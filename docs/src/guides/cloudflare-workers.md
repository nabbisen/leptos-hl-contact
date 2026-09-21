# Cloudflare Workers

The form's server path runs on Cloudflare Workers: a Leptos server compiled
for `wasm32-unknown-unknown`, with `leptos_axum`'s `wasm` feature and no tokio
runtime.  This page covers what differs from
[Axum Integration](./axum-integration.md).

## What works

| Feature | On Workers |
|---------|------------|
| `ssr` | supported |
| `form-token` | supported, without and with cookie binding |
| `challenge-http` | supported: the same request through the global `fetch` |
| `axum-helpers` | supported |
| `delivery-timeout` | supported: a JavaScript timer instead of tokio's |
| `smtp-lettre` | **not supported**: lettre's transport needs tokio and native TLS |

Delivery on a Worker is your own backend, such as a mail provider's HTTP
API; see [Writing your own backend](./delivery-backends.md#writing-your-own-backend).

## Dependencies

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
leptos = { version = "0.8", features = ["ssr"] }
leptos_axum = { version = "0.8", default-features = false, features = ["wasm"] }
axum = { version = "0.8", default-features = false }
leptos-hl-contact = { version = "0.8", default-features = false, features = [
    "ssr",
    "form-token",
    "challenge-http",
    "axum-helpers",
    "delivery-timeout",
] }
worker = { version = "0.8", features = ["http", "axum"] }
tower-service = "0.3"
```

`leptos_axum`'s default features are native-only.  `axum-helpers` does not
turn them on, so the `wasm` feature above is the whole choice.

**No build flag is needed.**  This crate enables `getrandom`'s `wasm_js`
feature for server builds on wasm32, which selects its JavaScript backend.

**Bundle size.**
- **Strip the debug names.**  A release build keeps the wasm `name` section,
  which can dominate the module: in one production Worker it was 55.1 MiB of
  a 59.1 MiB module, and the stripped module was 3.9 MiB.  Set
  `strip = "symbols"` in your release profile, or per build with
  `CARGO_PROFILE_RELEASE_STRIP=symbols`.
- **A measured build.**  Stripped, a production Worker with all five features
  and Turnstile measured about 4.3 MiB uncompressed and 1.5 MiB gzipped.  The
  challenge added about 25 KiB uncompressed, and the form token and the
  delivery deadline under 2 KiB; most of a Worker is Leptos and the
  application.  (One integrator's application, measured with
  `wrangler deploy --dry-run`, 2026-09-17; not a benchmark.)
- **Cloudflare's limit** is 64 MiB uncompressed on both the Free and Paid
  plans, with no compressed limit, though a larger bundle can slow startup
  ([Workers limits, "Worker size"](https://developers.cloudflare.com/workers/platform/limits/),
  checked 2026-09-16).
- **Build for size.**  Also build with `--release` and the usual size settings
  in your release profile: `opt-level = "z"` or `"s"`, `lto = true`,
  `codegen-units = 1`.

## Delivery on a Worker

On a Worker, a delivery's future need not be `Send`, so it may hold
JavaScript values, such as a pending `fetch`, across an `.await`.  Return the
`DeliveryFuture` alias, which drops `Send` on a wasm32 server and keeps it
natively:

```rust,ignore
use leptos_hl_contact::{ContactDelivery, ContactDeliveryError, ContactInput, DeliveryFuture};

/// Sends the message through your mail provider's HTTP API.
pub struct MailApiDelivery {
    pub api_key: String,
}

impl ContactDelivery for MailApiDelivery {
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_> {
        Box::pin(async move {
            // Call the API with `worker::Fetch` here.  On a Worker this
            // future need not be `Send`: it may hold JavaScript values
            // across an `.await`.
            let _ = (&self.api_key, input);
            Err(ContactDeliveryError::Configuration(
                "mail API not wired up yet".into(),
            ))
        })
    }
}
```

The type itself must still be `Send + Sync`, because Leptos context requires
it.  The same contract applies as for any backend: never put the submission
in an error's text.

## The context closure

The same values as on a native server, in the one closure passed to
`leptos_routes_with_context`.  Here the secrets come from the Worker's
environment:

```rust,ignore
use std::{sync::Arc, time::Duration};

use axum::http::request::Parts;
use leptos::prelude::*;
use leptos_hl_contact::{
    ChallengeClientIp, ChallengeContext, ChallengePolicy, ChallengeProvider,
    ContactDeliveryContext, DeliveryTimeout, FormTokenConfig, FormTokenContext,
    HttpChallengeVerifier, issue_form_token,
};

/// Secrets read from the Worker's environment on each request.
pub struct Secrets {
    pub form_token: Vec<u8>,
    pub turnstile: String,
    pub mail_api_key: String,
}

pub fn contact_context(secrets: Secrets) -> impl Fn() + Clone + Send + Sync + 'static {
    let delivery: ContactDeliveryContext = Arc::new(DeliveryTimeout::new(
        MailApiDelivery {
            api_key: secrets.mail_api_key,
        },
        Duration::from_secs(10),
    ));
    let token_config: FormTokenContext = Arc::new(FormTokenConfig::new(secrets.form_token));
    let challenge = ChallengeContext {
        verifier: Arc::new(HttpChallengeVerifier::new(
            ChallengeProvider::Turnstile,
            secrets.turnstile,
        )),
        policy: ChallengePolicy::default(),
    };

    move || {
        provide_context(Arc::clone(&delivery));
        provide_context::<FormTokenContext>(Arc::clone(&token_config));
        provide_context(issue_form_token(&token_config));
        provide_context(challenge.clone());

        // The visitor's IP, from the header Cloudflare sets on every request.
        let ip = use_context::<Parts>().and_then(|parts| {
            parts.headers.get("CF-Connecting-IP")?.to_str().ok()?.parse().ok()
        });
        if let Some(ip) = ip {
            provide_context(ChallengeClientIp(ip));
        }
    }
}
```

Cookie binding, a server policy and a success page are added exactly as in
[All context values together](./axum-integration.md#all-context-values-together).

## The Worker entry point

The router is built inside the `fetch` event, where the environment is
available:

```rust,ignore
use axum::{
    Router,
    body::Body,
    http::{Method, Response, StatusCode},
};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_service::Service;
use worker::{Context, Env, HttpRequest, event};

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> worker::Result<Response<Body>> {
    // Rate limiting, before the router: only submissions count.
    if req.method() == Method::POST {
        let key = req
            .headers()
            .get("CF-Connecting-IP")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("unknown")
            .to_owned();
        if !env.rate_limiter("CONTACT_RATE_LIMITER")?.limit(key).await?.success {
            return Ok(Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(Body::empty())
                .expect("a static response"));
        }
    }

    let secrets = Secrets {
        form_token: env.secret("FORM_TOKEN_SECRET")?.to_string().into_bytes(),
        turnstile: env.secret("TURNSTILE_SECRET")?.to_string(),
        mail_api_key: env.secret("MAIL_API_KEY")?.to_string(),
    };

    let options = LeptosOptions::builder().output_name("site").build();
    let routes = generate_route_list(App);
    let mut router = Router::new()
        .leptos_routes_with_context(&options, routes, contact_context(secrets), {
            let options = options.clone();
            move || shell(options.clone())
        })
        .with_state(options);

    Ok(router.call(req).await?)
}
```

`App` and `shell` are your application's root component and HTML shell, as
on a native server.

> **About these snippets.**  They are fenced `rust,ignore` because they need
> `worker`, `leptos_axum` and a wasm32 target, which the book's own test run
> does not have.  When this guide was written, all three snippets, with the
> dependencies above, were checked together in a
> scratch crate with `cargo check --target wasm32-unknown-unknown`.

## The visitor's IP

Cloudflare sets `CF-Connecting-IP` to the address of the client connecting
to it
([HTTP headers](https://developers.cloudflare.com/fundamentals/reference/http-headers/)).
The context closure above passes it to challenge verification as
`ChallengeClientIp`; `HttpChallengeVerifier` sends it to the vendor as
`remoteip`.

**The crate never reads a header itself.**  Which header can be trusted
depends on what is in front of the application.  Behind Cloudflare it is
`CF-Connecting-IP`; elsewhere the same header could be sent by anyone, and a
crate that guessed would accept a spoofed address.  You know your proxy, so
you choose.

The IP is personal data.  The crate never logs it, and `Debug` redacts it.
See [The visitor's IP](../security/challenge.md#the-visitors-ip).

## Rate limiting

Use the Workers
[Rate Limiting binding](https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/),
checked in the entry point **before** the router, so a limited request never
reaches the form's server function.  Declare it in `wrangler.toml`:

```toml
[[ratelimits]]
name = "CONTACT_RATE_LIMITER"
namespace_id = "1001"

  [ratelimits.simple]
  limit = 5
  period = 60
```

- **The period** is 10 or 60 seconds.
- **Not an exact count.**  Cloudflare describes the binding as "permissive,
  eventually consistent, and intentionally designed to not be used as an
  accurate accounting system", with "a unique limit per Cloudflare location"
  for each key (binding documentation, linked above, checked 2026-09-17).
- **The key.**  The entry point above keys by `CF-Connecting-IP`, because a
  contact form has no account to key by.  Cloudflare's documentation advises
  against IP keys in general, since many people can share one address (an
  office, a mobile carrier).
  - **Keep the limit generous** enough for a shared address.
  - **Count only `POST`**, as above.
  - **Key by something better** if your site has it.

**Verify it after deploying.**  Send more `POST`s than the limit from one
address to the deployed Worker, and expect a `429`.  `wrangler dev` is not
enough: in one integrator's production Worker, 27 `POST`s from one address in
about a minute got no `429` at a limit of 5 per 60 s, while `wrangler dev`
did refuse.

**A rate-limiting rule** for the zone, set in the Cloudflare dashboard, is
the alternative, or a second layer
([rate limiting rules](https://developers.cloudflare.com/waf/rate-limiting-rules/),
checked 2026-09-17).

The same layers as on a native server still apply: see
[Hardening](../security/hardening.md).

## The delivery deadline

`DeliveryTimeout` works on Workers.  The deadline is a JavaScript timer, and
a delivery that finishes first clears it.

**Choosing the limit.**  Waiting on a network call uses no CPU time, and a
Worker keeps running while the client stays connected
([Workers limits](https://developers.cloudflare.com/workers/platform/limits/)).
So the practical bound is how long a visitor will wait.

- **The context closure above uses 10 seconds.**
- **A timeout is logged** as `contact form delivery timed out`, with the
  limit only; see [Logs](#logs).
- **If the visitor disconnects,** Cloudflare may cancel the request's
  remaining work, so a slow delivery can be cut off without an answer.
- **When the deadline passes,** the visitor is told the message may have
  been sent (`delivery_timeout`).

`HttpChallengeVerifier` has its own five-second limit (`with_timeout`) on
Workers as well.

## Logs

**Why nothing prints.**  The crate logs through `tracing`.  A Worker usually
installs a `log` logger, such as `console_log`, and no `tracing` subscriber,
so the crate's lines print nothing.

**The fix.**  Enable `tracing`'s `log` feature in your Worker's crate:

```toml
tracing = { version = "0.1", features = ["log"] }
```

With that feature and no `tracing` subscriber active, each `tracing` event
is emitted as a `log` record, which your logger prints
([tracing: "Emitting `log` Records"](https://docs.rs/tracing/0.1.44/tracing/#emitting-log-records),
0.1.44, checked 2026-09-16).

**What that shows.**  Without it you do not see the operator's side of a
failure:
- `contact form delivery timed out` from the delivery deadline;
- `challenge verifier unavailable`;
- the delivery error's transport detail;
- a missing context value.

The sections above and below rely on these lines.

## Testing locally

Run the Worker with
[`wrangler dev`](https://developers.cloudflare.com/workers/wrangler/commands/workers/);
[workers-rs](https://github.com/cloudflare/workers-rs) describes the Rust
build.

**What to try.**  Enable the `log` feature first (see [Logs](#logs)), then
submit the form:
- with and without JavaScript;
- with a challenge;
- with a delivery that fails.

Without JavaScript, a refused submission comes back through a redirect with
an `__err` query parameter; see
[The error redirect without JavaScript](./axum-integration.md#the-error-redirect-without-javascript).

**What this project tests.**
- **CI** builds and lints the Workers feature set for wasm32 on every push.
- **The wasm32 server paths** run in headless Chrome: the form token's
  clock, the delivery deadline's timer, and challenge verification over a
  stubbed `fetch` (see [Testing](../development/testing.md#worker-tests)).
- **Not in CI:** the workerd runtime, which is what `wrangler dev` and a
  deployed Worker run.
  - **0.7.0** was tested on workerd by an integrator, on a pre-release and in
    production.
  - **Later releases** rely on the headless-Chrome suite above and on
    integrators' reports.
  - If something behaves differently on workerd, please open an issue.
