# RFC 008 — Test strategy

**Status.** Accepted — 2026-09-13.  Milestone M4 theme and both open
questions decided by the owner the same day (see §Owner decisions).
**Handoffs.** [`../handoffs/008-test-strategy/README.md`](../handoffs/008-test-strategy/README.md)
**Tracks.** Roadmap P-15.  Requirements NFR-TEST-02 (Partial), NFR-TEST-03
(Gap).
**Touches.** A new `crates/leptos-hl-contact/tests/` integration suite, dev-
dependencies, `.github/workflows/ci.yml`, `docs/src/development/testing.md`.
No change to the published crate's code or dependencies.

## Summary

The crate has about two hundred unit tests and no test that exercises it the
way an integrator's server does.  Every behaviour that crosses the HTTP
boundary — the honeypot's indistinguishable success, the success redirect on
every outcome, the no-JavaScript error round trip, the form token and cookie,
the challenge decision table — was proven by hand in review, with curl and
headless-browser transcripts that nothing re-runs.  This RFC adds the missing
layers so those proofs become tests, and ties each MUST requirement to the
tests that cover it.

## Motivation — what exists and what does not (measured 2026-09-13)

| Layer | Today |
|-------|-------|
| Pure logic (validation, codes, token verification, decision tables, timing helpers, cookie parsing) | ~200 unit tests in `src/<module>/tests.rs`; strong |
| SSR rendering of `ContactForm` | a few string-render tests in `components/tests.rs` |
| `submit_contact` over HTTP | **none**; no test calls it, no `tests/` directory |
| No-JavaScript round trip (`302` → `__err` → rendered field errors) | **none** |
| Hydrate-only browser logic (focus, token acquisition and refresh, explicit widget rendering, v3 submit script) | **none**; CDP recordings in review folders only |
| Live vendor endpoints | 4 `#[ignore]` tests, run by hand (P-26) |
| CI | fmt, clippy (two feature sets), `cargo test --all-features`, `doc -D warnings`; examples compile-checked |

Cases reviews already assigned here: the 0.4 `csrf_token` field still
accepted; the component rendering an error code end to end; the
field-error / banner routing pair; the one-context-closure routing property
(RFC 007); every successful outcome applying the success redirect (P-31, a
regression that shipped in 0.4.0 precisely because no such test existed).

## Goals

- Every behaviour a review verified by transcript at the HTTP boundary is a
  test that runs on every push.
- Each MUST requirement names the tests that cover it, and a requirement with
  none is visible as such.
- Browser-only logic has automated coverage.
- Tests stay deterministic and offline; nothing on every push contacts a
  vendor.

## Non-goals

- Code-coverage percentages as a gate.
- Live vendor tests in CI (P-26, a separate owner decision).
- Testing the example applications' own middleware beyond compiling them.
- Replacing unit tests: the new layers complement them.

## Design

### D1 — Four layers, each with one job

| Layer | Where | Tool | Proves |
|-------|-------|------|--------|
| **L1 unit** | `src/<module>/tests.rs` (unchanged rule) | `cargo test` | pure logic |
| **L2 server integration** | `tests/server/*.rs` | `cargo test`, in-process HTTP | everything observable in an HTTP response |
| **L3 browser** | `tests/browser/*.rs`, `wasm32-unknown-unknown` | `wasm-bindgen-test`, headless Chromium | hydrate-only behaviour |
| **L4 live** | existing `#[ignore]` tests | by hand | vendor contracts (P-26) |

A behaviour is tested at the lowest layer that can observe it, and only
there.  The integration `tests/` directory follows the project's line-count
splitting rule for its files.

### D2 — L2: the harness is the integrator's server

A test builds the router **exactly as the documentation tells integrators
to**: one context closure passed to `leptos_routes_with_context` (RFC 007),
a minimal `App` containing `ContactForm`, and no manual server-function
route.  Requests go through `tower::ServiceExt::oneshot`; no socket, no port.

Test doubles, in `tests/server/support.rs`:

- `RecordingDelivery` — counts deliveries and keeps the last input, so
  "delivered or not" is asserted directly rather than inferred from logs.
- `RecordingRedirect` — a `ContactSuccessRedirect` whose executor records.
- `ScriptedVerifier` — a `ChallengeVerifier` returning a given result.
- `FixedFilter` — a `ContactFilter` returning a given decision.
- Helpers to render the page, extract `form_token` and the binding cookie,
  and submit in either **no-JS form** (`Accept: text/html`, expect `302`) or
  **fetch form** (expect the server-function body and redirect header).

Every case runs in both forms where both exist, because the 0.4.0 regression
differed only in headers.

**Spike first.**  `#[server]` functions are registered by `server_fn` through
`inventory`, a registry populated at link time (`server_fn` 0.8.13,
`lib.rs:862`, `870`, `992`).  At acceptance the architect wrongly marked
this mechanism unconfirmed; handoff 01's spike and a second source check
confirmed it on 2026-09-13.  They must be registered inside an
integration-test binary for the router to serve them.  Handoff 01 proves
this with one passing request before writing the matrix; it did.  If registration does not happen, the fallback is to call
`leptos_axum::handle_server_fns_with_context` directly with a built request,
which exercises the same server-function code without the route table; the
routing property (RFC 007) would then be asserted separately.

### D3 — L2 case matrix (the minimum)

| Area | Cases |
|------|-------|
| Happy path | valid submission delivered once; `Ok`; success redirect applied when configured, inline otherwise; a delivery error reaches the client only as `delivery_failed` and is logged with its detail (NFR-TEST-03, FR-SUB-09; added at the handoff 01 review) |
| Validation | each rule rejects with its field code; the no-JS round trip renders the field text from labels after the `302`; field errors never also raise the banner; a banner error never sets `aria-invalid` |
| Honeypot / silent outcomes | honeypot and `SilentDrop` produce **the same status, `Location` and redirect header** as a delivered submission, with zero deliveries (T18) |
| Policy | `require_subject`, message limit in characters |
| Form token | missing config fails closed; missing, malformed, expired, too-young token; the 0.4 `csrf_token` field accepted; binding: missing cookie, mismatched cookie, `__Host-` name, nonce reused across renders, token endpoint reuses the nonce |
| Challenge | all seven decision-table rows through `ScriptedVerifier`, including the hyphenated field names |
| Filter | `Reject` → `rejected` code, chain short-circuit |
| Configuration | missing delivery context → `not_configured`; the one-closure routing property |
| Logging | a captured subscriber shows no message body, email, token or secret in any event for the cases above |

### D4 — L3: browser tests

`wasm-bindgen-test` in headless Chromium, mounting `ContactForm` into a test
document with server functions answered by a stubbed `fetch`.  Minimum:

- focus moves to the first invalid field after an error payload;
- the hidden token survives a failed submission (RFC 002);
- with `token_refresh_secs = None`, no request to the token endpoint;
  with `Some`, an empty field acquires once; a mounted overdue token refreshes
  once; a fetched token is scheduled from arrival (a controllable clock);
- a late widget element triggers explicit `render` on a stubbed vendor
  global, and a vendor script is inserted into `<head>` only when absent.

### D5 — Traceability

`docs/src/development/testing.md` gains a table: each MUST requirement in
the Requirements Specification → the test names that cover it.  Integration
tests carry the requirement IDs in their doc comments.  A requirement with no
test is listed with "none" rather than omitted, which satisfies NFR-TEST-02's
"derived from the specification" in a form a reviewer can check.

### D6 — CI

- L2 runs inside the existing `cargo test --all-features` step; no new job.
- L3 runs in a new `browser` job on **every push** (owner decision).
- Mutation testing (`cargo-mutants`) runs **once per milestone, before the
  release candidate**, as information for the architect's readiness report,
  never as a gate (owner decision); see the release process.
- L4 stays manual.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Keep proving HTTP behaviour by review transcripts | The 0.4.0 honeypot regression is the cost: a transcript proves one commit |
| Spawn the example binary and curl it in CI | Slower, needs ports and env, tests the example's middleware rather than the crate |
| Playwright or CDP scripts for L3 | Adds a Node toolchain to the repository; `wasm-bindgen-test` stays in Rust and in `cargo` |
| Coverage percentage gate | Rewards test count over the right tests; D5's table is the check that matters |

## Compatibility

None for integrators: tests and dev-dependencies only.  New dev-dependencies
expected: `tower` (`util`), `http-body-util`, `wasm-bindgen-test`, and a
`tracing` capture layer; none enters the published dependency tree.

## Security considerations

The suite turns five security properties that are currently guarded only by
review — T5 routing, T17 cookie tossing, T18 detection oracle, T19 fail-closed
vendor errors, and log hygiene (T9) — into regression tests.  No test uses a
real secret or contacts a vendor.

## Acceptance criteria

- NFR-TEST-03 Met: every case in D3 exists and passes, in both request forms
  where both exist.
- NFR-TEST-02 Met: the D5 table exists, and every MUST requirement is listed.
- The five recorded cases from Motivation each have a named test.
- L3 cases in D4 pass locally, and in CI if the owner so decides.
- Gates and CI green; total `cargo test` time reported before and after.

## Implementation boundaries

Three handoffs: **01** L2 harness, spike, and the D3 matrix; **02** L3
browser tests; **03** the traceability table and requirement IDs.  01 comes
first; 02 and 03 may follow in parallel.

## Owner decisions (2026-09-13)

1. Browser tests run in CI **on every push**.
2. Mutation testing runs **once per milestone, before the release candidate**,
   informational only.

## Open questions as originally put

1. **Browser tests in CI.**  Run the L3 job on every push (GitHub Actions
   minutes on a public repository; adds a few minutes per run), or locally
   only at first.  Architect recommends **every push**: the browser logic is
   where the least-covered and most intricate code lives.
2. **Mutation testing.**  `cargo-mutants` finds tests that pass without
   checking anything (the project `.gitignore` already expects its output).
   Run it periodically and read the report, or not at all.  Architect
   recommends **periodically, informational, never a gate** — for example
   once per milestone, before the release candidate.

## Release implications

No release of its own; the suite ships with whatever release follows.
