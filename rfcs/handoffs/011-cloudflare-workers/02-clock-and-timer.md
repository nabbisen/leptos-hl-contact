# Handoff 011-02 — A JavaScript clock and timer

**RFC.** [RFC 011](../../done/011-cloudflare-workers.md) D4, D7
**Roadmap.** P-23
**Requirements.** FR-VAL-06, FR-ABUSE-03, FR-ABUSE-13, FR-DEL-08, NFR-PORT-02
**Depends on.** Handoff 01 merged.

## Goal

On a wasm32 server:
- **The form token** issues and verifies without panicking.
- **`DeliveryTimeout`** enforces its deadline with a JavaScript timer.

Native behaviour is unchanged.

## Change scope

### 1. One clock for the form token (D4)

**`src/form_token.rs`** reads the time in two places today: verification
(line 285, `unwrap_or(0)`) and `sign_token` (line 417,
`.expect("system clock before Unix epoch")`).  Replace both with one private
function:

```rust,ignore
/// Seconds since the Unix epoch.  A clock before 1970 reads as 0.
fn now_unix_secs() -> u64 { … }
```

- **Native:** `SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)`.
- **wasm32 server:** `(js_sys::Date::now() / 1000.0) as u64`.

**One intentional change:**
- **Before.**  Issuing a token with a clock before 1970 panicked.
- **After.**  It signs timestamp 0, which verification rejects as expired.
  The failure becomes fail-closed instead of a crash.

Add a CHANGELOG *Changed* line for it.

**Dependencies.**  `js-sys` becomes a wasm32-only optional dependency.
`form-token` and `delivery-timeout` enable it.

### 2. A private wasm timer (D7)

**New private module** `src/wasm_timer.rs`, compiled on a wasm32 server
only:
- **`fn sleep(duration) -> Sleep`.**  A future that resolves after
  `duration`.  It uses `setTimeout` taken from `js_sys::global()`: a
  Worker's global scope has no `window`.
- **Dropping `Sleep`** calls `clearTimeout`, so no timer outlives its use.
- **Dependencies:** `wasm-bindgen` (`Closure`) and `wasm-bindgen-futures`,
  as wasm32-only optional dependencies, enabled by `delivery-timeout`.
  Handoff 03 enables them for `challenge-http` too.

### 3. `with_deadline` on a wasm32 server (D7)

**`src/delivery/timeout.rs`.**  Replace handoff 01's TODO.  On a wasm32
server, race the delivery against `wasm_timer::sleep(limit)`:
- **The delivery finishes first:** return its result.  Dropping `Sleep`
  clears the timer.
- **The timer finishes first:** drop the delivery and return
  `Err(ContactDeliveryError::Timeout(limit))`.

**How to race.**  Use `std::future::poll_fn`, polling the delivery first,
then the timer.  No `futures` crate.

**Native:** `tokio::time::timeout`, unchanged.

**`delivery-timeout`'s features** become `["ssr", "dep:tokio", "tokio/time"]`
plus the wasm32 dependencies above.  tokio remains native-only (handoff 01).

### 4. Carried over from the handoff 01 review

- **`src/filter.rs` rustdoc example (`MaxLinks`).**  Return `FilterFuture<'_>`
  instead of spelling out the native future type.  A Workers integrator who
  copies the example must not get a type mismatch.
- **`.github/workflows/ci.yml`.**  The Workers step's comment says "no
  tokio"; make it "no tokio runtime".  `leptos_axum` compiles tokio in
  unconditionally.
- **The interim `with_deadline`.**  Remove it (§3).
  `git grep -n "TODO(011-02)"` must print nothing.

## Tests

**L1, native:** `form_token::now_unix_secs_matches_the_system_clock`.  It is
within two seconds of `SystemTime::now()`.

**`tests/worker/` (headless Chrome, a server-feature build):**
- **`clock::a_token_issued_now_verifies`:** a `FormTokenConfig` with
  `with_min_age(0)`; `issue_form_token`, then `verify_form_token` succeeds.
  This proves the clock does not panic and reads the present.
- **`clock::a_token_is_too_young_before_its_minimum_age`:** with
  `with_min_age(3600)`, verification returns `TooYoung`.
- **`timer::a_delivery_that_never_finishes_times_out`:**
  - `DeliveryTimeout::new(Never, 50 ms)`;
  - returns `Timeout(50 ms)`;
  - takes at least 50 ms, measured with `js_sys::Date::now()`.

  A real 50 ms wait is allowed here only: this suite cannot pause a
  JavaScript clock, and 50 ms is negligible.
- **`timer::a_delivery_that_finishes_in_time_returns_its_result`:** `Ok` and
  a `Transport` error pass through.

**CI.**  In the `browser` job, add a second step after the existing one:

```yaml
      - name: worker tests (server features on wasm32)
        env:
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER: wasm-bindgen-test-runner
          RUSTFLAGS: --cfg getrandom_backend="wasm_js"
        run: CHROMEDRIVER="$CHROMEWEBDRIVER/chromedriver" cargo test -p leptos-hl-contact --target wasm32-unknown-unknown --no-default-features --features ssr,form-token,delivery-timeout --test worker
```

**If a server-feature build cannot run in the browser**, report the exact
failure.  Do not work around it by gating it out.  The runtime rows then move
to the reflerd.com report, which the architect decides.

**Traceability.**  The FR-VAL-06 row gains the clock tests, marked "wasm32
server, headless Chrome".

## Acceptance

1. **Native gates, both suites and the examples** pass.
   `cargo test --all-features` counts are unchanged, plus one unit test.
2. **The new browser-job step** passes locally with runner 0.2.121 and in
   CI.  Report the counts.
3. **The Workers check from handoff 01** still passes.
4. **Deliberate breaks:**
   - **A.**  On wasm32, make `now_unix_secs` return `0`.  The
     `a_token_issued_now_verifies` test fails as `Expired`.
   - **B.**  On wasm32, await the delivery without the timer.  The worker
     timeout test hangs; bound it with a process timeout and report how.

   Restore both.
5. **No timer leak.**  Show the `clearTimeout` call in the `Drop` impl.
   Explain why a finished delivery cannot leave a pending timer.

## Review request

`.git-exclude/review-request/011-cloudflare-workers/02-clock-and-timer.md`
