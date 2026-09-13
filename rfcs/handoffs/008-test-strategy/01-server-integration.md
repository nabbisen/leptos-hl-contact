# Handoff 01 — Server integration suite

**RFC.** [008](../../accepted/008-test-strategy.md), D2 and D3.
**Requirements.** NFR-TEST-03; regression guards for T5, T9, T17, T18, T19.

## Purpose

Turn every behaviour reviews proved by transcript at the HTTP boundary into a
test that runs on every push.

## Change scope

- New `crates/leptos-hl-contact/tests/server/` (`main.rs` plus one file per
  area, split by the project's line-count rule) and
  `tests/server/support.rs`.
- `crates/leptos-hl-contact/Cargo.toml` `[dev-dependencies]` only: `tower`
  (`util`), `http-body-util`, and a `tracing` capture layer such as
  `tracing-subscriber` with `registry` — dev-only, report each.
- `docs/src/development/testing.md`: a "Server integration suite" section.
- `CHANGELOG.md` `[Unreleased]` → *Documentation*/*Added* as appropriate.

## Explicit non-change scope

No change to `src/`.  If a test cannot be written without changing crate code,
stop and report: that is a design finding, not a test task.

## Required implementation

### Step 0 — spike, reported before anything else

Build, in one test, a router **exactly as the docs instruct integrators**:

- `LeptosOptions::builder().output_name("contact-test").build()`;
- a minimal `App` rendering `ContactForm` at `/contact`;
- `generate_route_list(App)` and `leptos_routes_with_context(&options,
  routes, context_closure, shell)` with a single context closure (RFC 007);
- no hand-written server-function route.

Send `POST /api/submit_contact` through `tower::ServiceExt::oneshot` and
assert a delivered submission.  **Record in the review request whether the
server functions were reachable through that router inside the test binary**,
and how you established it.  RFC 008 D2 said they register "through
`inventory`"; the architect could not confirm that mechanism in `server_fn`
0.8, so treat it as unknown.  If they are not reachable, use the RFC's
fallback — `leptos_axum::handle_server_fns_with_context` with a built
request — and write one separate test that asserts the routing property
another way; say which you did.

### Step 1 — `support.rs`

- `RecordingDelivery` (count and last input), `RecordingRedirect`,
  `ScriptedVerifier`, `FixedFilter`.
- `Harness::new(config)` building the router with whichever context values a
  test needs; `Harness::render()` returning page HTML, `form_token` and the
  binding cookie; `Harness::submit_nojs(fields)` and
  `Harness::submit_fetch(fields)` returning status, headers and body.
- A log capture that collects every event's formatted fields for the test.

### Step 2 — the D3 matrix

Implement every row of RFC 008 D3.  Where a behaviour exists in both request
forms, write it as one test that runs both and asserts both.  Required named
cases, each with a doc comment citing its requirement or threat ID:

- `a_silent_outcome_is_indistinguishable_from_delivery` — honeypot and
  `SilentDrop` versus delivery, with and without a success page: identical
  status, `Location` and `serverfnredirect`; deliveries 1 / 0 / 0 (T18,
  FR-ABUSE-09).
- `field_errors_round_trip_without_javascript` — `302`, follow `Location`,
  the rendered page contains the field text from labels and
  `aria-invalid="true"`, and no assertive banner.
- `a_banner_error_sets_no_field_error` — token failure: banner, zero
  `aria-invalid`.
- `the_0_4_csrf_token_field_is_accepted` — a compatibility promise with a
  removal date; its doc comment names the minor that removes it.
- `context_in_the_one_closure_reaches_submit_contact` (RFC 007).
- `binding_rejects_a_tossed_bare_cookie` and
  `binding_reuses_the_browser_nonce_across_renders` (T17).
- `challenge_decision_table_rows_1_to_7` through `ScriptedVerifier`,
  including the hyphenated field names on the wire.
- `no_personal_data_or_secret_is_logged` — run a representative set of the
  cases above under the log capture and assert that no event contains the
  submitted name, email, message, token, cookie value or secret (T9).

## Required tests

The matrix above; nothing else is required.  Unit tests in `src/` are not
modified.

## Acceptance criteria

- Step 0 result stated plainly, with evidence.
- Every D3 row has at least one test; the review request lists row → test
  name.
- Deleting the success-redirect call from the honeypot path (a local,
  uncommitted change) makes `a_silent_outcome_is_indistinguishable_from_delivery`
  fail — paste that failure, then restore.  Same for removing the nonce reuse
  and `binding_reuses_the_browser_nonce_across_renders`.
- Gates green on both toolchains; wall time before and after reported; CI
  green.

## Prohibited shortcuts

Spawning the example binary; asserting on log text to prove behaviour that
has an HTTP-visible effect; `sleep` for the minimum age (construct a token
with a past timestamp through the public API instead, or configure
`with_min_age(0)` where the case is not about age).

## Known risks

- `generate_route_list` may need a Tokio runtime or reactive owner; build
  the harness inside `#[tokio::test]`.
- Test binaries share server-function registrations; keep context per test,
  never global.

## Required evidence

Step 0 finding; row → test table; the two deliberate-break failures; gate
output; wall times; CI URL.
