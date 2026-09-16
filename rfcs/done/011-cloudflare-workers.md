# RFC 011 — Cloudflare Workers as a supported server target

**Status.** Accepted — 2026-09-15.  The owner accepted the RFC with the
recommended option on each open question (§Owner decisions), having
decided the same day that Cloudflare Workers is a supported server target
(P-23), delivered as milestone M5 → 0.7.0, with the reflerd.com team as
runtime testers.
**Handoffs.** [`../handoffs/011-cloudflare-workers/README.md`](../handoffs/011-cloudflare-workers/README.md)
**Tracks.** Roadmap P-23, P-40.  Requirements NFR-PORT-02 (Decision →
Planned), NFR-PORT-01, FR-ABUSE-10 and -12, NFR-PRIV-02.  RFC 005's
redirect guarantee.
**Touches.** `Cargo.toml` (target-specific dependencies, feature wiring),
`delivery.rs`, `challenge.rs`, `challenge/http.rs`, `filter.rs`,
`form_token.rs`, `delivery/timeout.rs`, `server.rs`, CI, documentation.
**Origin.** A request from the reflerd.com team, 2026-09-13.  They run
their site's server as a Worker and want Turnstile, the form token and the
success redirect there.

## Summary

Today a Leptos server built for Cloudflare Workers can use the crate's core
(`ssr`) and nothing else.
- **Turnstile verification does not compile.**
- **The form token compiles, then panics** on its first clock read.
- **The Axum helpers do not compile.**
- **Every extension trait forces a `spawn_local` bridge,** because it
  demands `Send` futures.

This RFC makes the server path work on Workers without changing native
behaviour:
- **`Send`:** future bounds relax on a wasm32 server build only.
- **Verification:** the built-in verifier uses `fetch` there, with the same
  refusal of redirects.
- **Clock:** the form token reads a JavaScript clock.
- **Timeout:** `DeliveryTimeout` uses a JavaScript timer.
- **Axum helpers:** they stop pulling in tokio.
- **Client IP:** verifiers may receive the visitor's IP.
- **Docs:** they say exactly what works where.

## Motivation — measured on 0.6.0 (architect, 2026-09-15)

`cargo check -p leptos-hl-contact --target wasm32-unknown-unknown --no-default-features --features <set>`,
with the application's `getrandom` `wasm_js` configuration:

| Features | Result | Cause |
|----------|--------|-------|
| `ssr` | compiles | — |
| `ssr,form-token` | compiles; **panics at run time** | `SystemTime::now()` in `form_token.rs` (285, 417) is `unimplemented` on `wasm32-unknown-unknown` |
| `ssr,challenge-http` | **4 errors** | reqwest's wasm build has no `redirect` module (`http.rs:75`); its futures are not `Send`, but `ChallengeVerifier::verify` returns a `Send` future (`http.rs:127`, `:146`) |
| `ssr,axum-helpers` | **fails** (`mio`) | `leptos_axum` and `axum` are depended on with default features, which bring in tokio's networking |
| `ssr,delivery-timeout` | compiles; **cannot run** | `tokio::time` needs a tokio runtime, and workerd has none |

**Constraints established by reading the framework source:**

1. **Server functions need `Send` futures on every target.**  `server_fn`
   0.8.13 declares its body as `impl Future + Send` (`lib.rs:287`), with no
   wasm exception.  So `submit_contact`'s own future must stay `Send`.
2. **Context needs `Send + Sync` values.**  Leptos's
   `provide_context<T: Send + Sync + 'static>` (`reactive_graph` 0.2.14,
   `owner/context.rs:203`) applies on every target.  So the extension
   *objects* (`Arc<dyn ContactDelivery>` and the others) must stay
   `Send + Sync`.  Only the futures they return can relax.
3. **reqwest's wasm client** can time out (through `AbortController`), but
   it cannot refuse redirects: it has no redirect policy.  Using it
   unchanged would let a redirect resend the vendor secret.  RFC 005
   forbids that.
4. **Workers' `fetch`** supports `redirect: "manual"`, which returns the 3xx
   response as-is, `"error"`, and `AbortSignal` (Cloudflare Workers
   Request documentation, checked 2026-09-15).
5. **A Worker isolate runs one thread.**  A value that is not `Send` never
   crosses threads there.  That makes `send_wrapper::SendWrapper` sound, and
   `server_fn` already uses it for its browser client.

## Design

### D1 — What "supported on Workers" means

**The target.**  A Leptos server compiled for `wasm32-unknown-unknown`, with `leptos_axum`'s `wasm` feature and no tokio runtime.  (tokio itself is
still compiled in, because `leptos_axum` depends on it unconditionally;
nothing starts a tokio runtime, and tokio's networking, `mio`, is absent.
Amended at the handoff 01 review, 2026-09-15.)  The term "server on wasm32"
below means `all(target_arch = "wasm32", feature = "ssr")`.  A browser
(`hydrate`) build never enables `ssr`, so none of this touches it.

| Feature | On Workers after this RFC |
|---------|---------------------------|
| `ssr` | supported (already) |
| `form-token` | supported, without and with cookie binding |
| `challenge-http` | supported (D3) |
| `axum-helpers` | supported (D6) |
| `delivery-timeout` | supported (D7) |
| `smtp-lettre` | **not supported**: lettre's transport needs tokio and native TLS.  Delivery on Workers is the integrator's own `ContactDelivery` |

**Verification:**
- **CI** builds and lints the Workers feature set,
  `ssr,form-token,challenge-http,axum-helpers,delivery-timeout`, for
  wasm32 on every push.
- **Runtime behaviour** on `wrangler dev` and on a live Worker is verified by
  the reflerd.com team before 0.7.0 is released; see owner decision 2.

### D2 — `Send` relaxes for futures only, on a wasm32 server

**Future aliases.**  Each extension trait names its future through a public
alias:

```rust,ignore
// delivery.rs
#[cfg(not(all(target_arch = "wasm32", feature = "ssr")))]
pub type DeliveryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + 'a>>;
#[cfg(all(target_arch = "wasm32", feature = "ssr"))]
pub type DeliveryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + 'a>>;

pub trait ContactDelivery: Send + Sync + 'static {
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_>;
}
```

The same pattern gives `challenge::VerifyFuture<'a>` and
`filter::FilterFuture<'a>`.

**The traits keep `Send + Sync + 'static`**, as constraint 2 requires.  A
Workers delivery holding only configuration is `Send + Sync`.  Its futures,
over Workers sockets or `fetch`, need not be.

**Inside `submit_contact`, on a wasm32 server**, each extension future is
awaited through `SendWrapper::new(…)`: the challenge verification, the
filter and the delivery.  The server function's future therefore stays
`Send` (constraint 1).
- **Dependency.**  `send_wrapper = { version = "0.6", features = ["futures"] }`,
  a wasm32-only optional dependency enabled with `ssr`.  It is already in
  the lock through `server_fn`.
- **Safety.**  `SendWrapper` panics if a value is used from another thread.
  That cannot happen in a Worker isolate (constraint 5).  The rustdoc states
  the assumption.

**Native builds see the same types.**  Each alias expands to exactly the
type written today, so an existing implementation that spells out the full
type still compiles.

**Step 0 of the handoff is a spike, reported before anything else.**  A
`#[server]` function awaiting a non-`Send` delivery future through
`SendWrapper` must compile for wasm32 with `leptos_axum`'s `wasm` feature.
If it does not, stop: the design goes back to review.

### D3 — `HttpChallengeVerifier` over `fetch` on a wasm32 server

**Native is unchanged:** reqwest, `redirect::Policy::none()`, a five-second
cap.

**On a wasm32 server** the same public type sends the same request through
the global `fetch`, taken from `js_sys::global()` so that a Worker's global
scope works:

| Property | Native | wasm32 server |
|----------|--------|---------------|
| Request | `POST`, form-encoded `secret`, `response`[, `remoteip`] | the same |
| Redirects | not followed; a 3xx is `Unavailable` | `redirect: "manual"`; a 3xx is `Unavailable` — **the secret is never resent** |
| Time limit | `timeout` (five seconds by default, `with_timeout`) | `AbortController` aborted by a timer; an abort is `Timeout` |
| Non-2xx, network error | `Unavailable` | `Unavailable` |
| Body | shared `parse_response` | shared `parse_response` |
| Empty secret | `Misconfigured`, nothing sent | the same |
| `with_verify_url`, `Debug` redaction | as today | the same |

**Dependencies.**  reqwest becomes a native-only dependency.  The wasm
build uses `web-sys`, `js-sys` and `wasm-bindgen-futures`, all already in
the lock.  No new crate.

**The URL.**  A browser's `"manual"` mode yields an opaque redirect with
status 0.  That is also non-2xx, so it is also `Unavailable`.  The mapping
does not depend on which runtime answers.

### D4 — A wasm-safe clock for the form token

`form_token` reads the time through one private function:
- **Native:** `SystemTime::now()`, as today.
- **wasm32 server:** `js_sys::Date::now()`, in milliseconds, divided down to
  seconds.

Nothing else in the token changes: TTL, minimum age, future skew, and
binding.  Binding stays optional; the reflerd.com site sets no cookies and
will use the token without it.

**Randomness.**  A Leptos server on wasm32 pulls in two `getrandom`
versions without their wasm backend.  Amended 2026-09-15 after the
handoff 01 spike, which found the second one.
- **`getrandom` 0.3** comes through `rand` 0.9: this crate's form token, and
  Leptos's `nonce` feature.  Since **0.3.4** it needs only the `wasm_js`
  feature; 0.3.0–0.3.3 also needed the `--cfg getrandom_backend="wasm_js"`
  flag (`getrandom-0.3.4/CHANGELOG.md`).
- **`getrandom` 0.4** comes through Leptos's `nonce` feature, which
  `leptos_axum` enables.  It needs only the `wasm_js` feature
  (`getrandom-0.4.2/src/backends.rs:172`).
- **The crate** enables `wasm_js` on both, as wasm32-only dependencies
  switched on by `ssr`.  The 0.4 entry is a renamed dependency whose
  comment says to drop it once Leptos enables the backend for server builds
  itself.  A browser (`hydrate`) build never enables `ssr` and is unchanged.
- **The application needs no build flag.**  The crate requires `getrandom`
  0.3.4, so the `wasm_js` feature it enables is always enough.  (Amended
  again at the handoff 04 review, 2026-09-15.  Handoff 04 found that a
  Workers build compiles without the flag; the earlier amendment had said the
  application must still pass it.)

### D5 — The visitor's IP for challenge verification (P-40)

**A request type and a default method, both additive:**

```rust,ignore
#[non_exhaustive]
pub struct ChallengeRequest<'a> {
    pub token: &'a str,
    pub remote_ip: Option<std::net::IpAddr>,
}

pub trait ChallengeVerifier: Send + Sync + 'static {
    fn verify(&self, token: &str) -> VerifyFuture<'_>;
    /// Default: ignores everything but the token.
    fn verify_request(&self, request: &ChallengeRequest<'_>) -> VerifyFuture<'_> {
        self.verify(request.token)
    }
}
```

**Where the IP comes from.**  The integrator provides it per request as
`ChallengeClientIp(IpAddr)` in the context closure, for example from
`CF-Connecting-IP` on Workers.  `submit_contact` calls `verify_request` with
it when present.
- **The crate never reads a header itself.**  Which header can be trusted
  depends on the proxy in front of the site.  A crate that guessed would
  accept a spoofed `X-Forwarded-For`.
- **`HttpChallengeVerifier`** sends `remoteip` when it is present.  All three
  vendors accept that parameter; the handoff confirms the hCaptcha and
  reCAPTCHA names against their documentation.
- **Logging.**  The IP is PII (Requirements §4) and is never logged.
- **Privacy docs.**  The per-vendor privacy notes (NFR-PRIV-02) gain one
  line: the visitor's IP is sent when the site provides it.

### D6 — `axum-helpers` without tokio

- **Dependencies.**  `leptos_axum` and `axum` become
  `default-features = false`.  `axum_helpers` uses only `axum::http`, which
  needs no axum feature.
- **The application chooses.**  A native Axum application already depends on
  `leptos_axum` with its defaults, because it calls
  `leptos_routes_with_context`, and features unify.  A Worker enables
  `leptos_axum`'s `wasm` feature.
- **Our native tests** add `leptos_axum` and `axum` with defaults as
  dev-dependencies, so the server suite is unaffected.
- **The handoff proves it three ways:** the server suite, both examples, and
  a native check of the crate with `axum-helpers` next to an application
  that enables the defaults.

### D7 — `DeliveryTimeout` without tokio

- **Native:** unchanged (`tokio::time::timeout`).
- **wasm32 server:** `with_deadline` races the delivery against a timer
  created with the global `setTimeout`.  It clears the timer when the
  delivery finishes first, so no timer outlives the request.  Expiry drops
  the delivery and returns `Timeout(limit)`, exactly as natively.
- **Features.**  tokio becomes a native-only dependency of this crate.  On
  wasm32 this crate adds no tokio dependency, and `mio` is absent.  (The
  proposal said `cargo tree -i tokio` would print nothing.  It cannot while
  `leptos_axum` depends on tokio unconditionally; amended at the handoff 01
  review, 2026-09-15.)

### D8 — Documentation

- **New guide: `guides/cloudflare-workers.md`.**
  - the dependency set (`leptos_axum` `wasm`, the `getrandom` flag);
  - which features work (D1's table);
  - the context closure on a Worker;
  - `ChallengeClientIp` from `CF-Connecting-IP`;
  - rate limiting with the Workers Rate Limiting binding;
  - delivery through the site's own backend, now with non-`Send` futures;
  - `DeliveryTimeout`.
- **`security/challenge.md`, CSP section.**
  - **Turnstile needs** `script-src https://challenges.cloudflare.com` and
    `frame-src https://challenges.cloudflare.com`.  A nonce on its
    `api.js` script propagates to what it loads, and `'strict-dynamic'` is
    supported but not required.  `connect-src 'self'` is needed only in
    pre-clearance mode (Cloudflare documentation, checked 2026-09-15).
  - **The same for hCaptcha and reCAPTCHA,** confirmed against their
    documentation by the handoff.
  - **The reCAPTCHA v3 inline script** carries the widget's nonce.
- **Production Checklist:** a Workers section covering client IP, rate
  limiting, the feature set, and CSP.
- **Feature Flags page:** a "Workers" column.

### D9 — Records

- **NFR-PORT-02 becomes a requirement:** "The server path (`ssr`,
  `form-token`, `challenge-http`, `axum-helpers`, `delivery-timeout`) MUST build for and run on Cloudflare Workers (wasm32, no tokio runtime).  Delivery
  there is the integrator's own backend."  Status Planned → Met (0.7.0).
- **Traceability.**  The table gains the row, with the CI step and the
  reflerd.com runtime report as its verification.
- **Architecture.**  The page describes the three cfg paths: native,
  browser, and wasm32 server.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Keep `Send` everywhere; document the `spawn_local` bridge | Every Workers integrator writes the same bridge for each trait, and the built-in verifier still cannot run |
| Drop `Send` from the futures on every target | `submit_contact`'s future must be `Send` natively (constraint 1); native multi-threaded servers would lose the guarantee |
| Relax on `target_arch = "wasm32"` alone | Browser (`hydrate`) builds would see different trait types from today, and a shared crate compiled for both targets could break for no gain.  The `ssr` condition confines it to servers |
| Drop `Send + Sync` from the trait objects | Leptos context requires `Send + Sync` values (constraint 2) |
| Use reqwest's wasm client for verification | It cannot refuse redirects, so it would weaken RFC 005's guarantee that the secret is never resent |
| An injectable clock on `FormTokenConfig` | A public API for a platform difference the crate can absorb privately; tests already construct past-dated tokens |
| Read `CF-Connecting-IP` inside the crate | Correct only behind Cloudflare; elsewhere the same approach invites spoofed headers.  The integrator knows its proxy |
| A built-in Workers delivery adapter | Not requested, and the reflerd.com team has its own.  It belongs with the HTTP delivery adapters (P-22).  See owner decision 1 |
| Run workerd in our CI | A Node or wrangler toolchain and more CI minutes.  Compile checks plus the reflerd.com runtime report first.  See owner decision 3 |

## Compatibility

**Native: no source change expected.**
- **Future aliases.**  They expand to today's types.
- **`verify_request`.**  It has a default implementation.
- **New types.**  `ChallengeRequest` and `ChallengeClientIp` are additions.

One edge is documented in the CHANGELOG: an application that enables
`axum-helpers` but does not depend on `leptos_axum` itself would now get
`leptos_axum` without its defaults.  Every Axum application using the form
calls `leptos_routes_with_context`, so it has that dependency.

**wasm32 server: new capability.**  An implementation compiled there that
spells out `+ Send` in its return type should switch to the alias.  Only
Workers users compile for that target, and today they cannot use the
extensions without a bridge.

**Release:** 0.7.0, a minor release, because it adds features.

## Security considerations

- **RFC 005 still holds on Workers.**  A redirect is never followed, so the
  secret never goes to a `Location`.  The time limit still holds, and an
  unreachable vendor still fails closed.
- **`SendWrapper`'s single-thread assumption** is true of Workers and
  documented.  A future multi-threaded wasm runtime would panic loudly,
  not misbehave silently.
- **The visitor's IP** reaches a vendor only when the site provides it, and
  is never logged.
- **Nothing server-side reaches a browser build.**  `challenge-http` and
  `form-token` imply `ssr`, which a `hydrate` build never enables.

## Testing

- **Step 0 spike** (D2), reported first.
- **CI:** `cargo check` and `clippy -D warnings` for the Workers feature set
  on wasm32, on every push.
- **Native suites unchanged,** plus:
  - a server test that `ChallengeClientIp` reaches the verifier's
    `verify_request`;
  - a unit test that `HttpChallengeVerifier` puts `remoteip` in the form
    body.
- **Browser harness:** the wasm32 fetch path of `HttpChallengeVerifier`
  under a stubbed `fetch`, where a server-feature build can run in headless
  Chrome.  It asserts `redirect: "manual"`, 3xx → `Unavailable`, abort →
  `Timeout`, and the form body.  If a server-feature build cannot run in
  the browser, the handoff says so, and the reflerd.com runtime report
  carries this row.
- **Deliberate breaks:** follow the redirect on wasm32; drop `SendWrapper`
  around the delivery.
- **reflerd.com runtime report before release,** on `wrangler dev` and the
  live Worker, with its Content Security Policy enforced:
  - Turnstile pass, fail and unavailable;
  - the form token with and without JavaScript;
  - the success redirect;
  - the delivery timeout.

## Owner decisions (2026-09-15)

Accepted with the recommended option on each question:

1. **Delivery on Workers stays the integrator's own backend in 0.7.0.**  No
   built-in Workers delivery adapter; that stays with P-22.
2. **The reflerd.com team tests from a git revision** of `main` before the
   release.  Nothing is published before 0.7.0 itself.
3. **No workerd or wrangler runtime job in our CI** for 0.7.0; revisit after
   the reflerd.com runtime report.

## Release implications

M5 → **0.7.0**, together with RFC 012 (honeypot without inline style).
Handoffs after acceptance, in order:
1. **01:** the spike, D2, D6, and the CI Workers check.
2. **02:** D4 and D7.
3. **03:** D3 and D5.
4. **04:** D8 and D9.

The release candidate waits for the reflerd.com runtime report.
