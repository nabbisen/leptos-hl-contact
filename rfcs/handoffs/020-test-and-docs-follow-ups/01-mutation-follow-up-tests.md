# Handoff 020-01 — The mutation follow-up tests

**RFC.** [RFC 020](../../done/020-test-and-docs-follow-ups.md) D1
**Roadmap.** P-38
**Requirements.** NFR-TEST-02/03, and the requirement each test cites

## Goal

Close the survivors the mutation runs have reported since 0.6.0.  **Tests
only.**

## The tests

Each row names the mutant it kills.  **Write the test, then show it failing
against that mutant**, then restore — the project's break-check rule, once
per row.

| # | Test | The mutant it kills |
|---|------|---------------------|
| 1 | the form token at **exactly** its TTL is still valid, and one second past is `Expired` | `form_token.rs` `replace > with >= in verify_form_token` (TTL) |
| 2 | a token at **exactly** the future-skew limit is accepted, one second beyond is refused | `form_token.rs` `replace > with >= in verify_form_token` (skew) |
| 3 | `Debug` assertions for `FormTokenIssuer`, `ChallengeContext`, `FilterChain` and `ResendDelivery` | the four `<impl Debug>::fmt → Ok(Default::default())` mutants |
| 4 | `provide_contact_delivery` puts the delivery in context, and the server function finds it | `axum_helpers.rs` `replace provide_contact_delivery with ()` |
| 5 | the `challenge unavailable` log event, and the missing-context one | `server.rs` `replace == with !=` and `delete match arm ContactErrorCode::ChallengeUnavailable` |
| 6 | **a 64,000-byte response body is accepted** (and one over the cap is still `Unusable`) | `http.rs` `replace * with + in MAX_RESPONSE_BODY` |

**Notes that decide where each test lives:**
- **1, 2, 3, 6** are unit tests, in the module's own `tests.rs`.
- **4 and 5** need the router or the log capture, so they belong in
  `tests/server/`, where `capture_logs()` is.
- **3** asserts what each `Debug` must and must not contain: the secret or
  key is `"<redacted>"`, and the probe value never appears.  Use a
  distinctive fake value.
- **6** must **not** be written as `assert_eq!(MAX_RESPONSE_BODY, 64 * 1024)`:
  that restates the definition.  Build a 64,000-byte body — a literal count,
  not the constant — and assert it is returned whole.

**Every test cites its requirement or threat ID** in its doc comment, as
`testing.md` requires.

## Also

- **`docs/src/development/testing.md`:** add each test to the traceability
  row it belongs to.
- **`CHANGELOG.md`:** nothing.  Tests only; say so in the request.

## Gates

The shared gates on both toolchains; the MSRV checks; both wasm suites with
counts; the examples with `--locked`; CI.  **Report the new counts** against
0.9.0's 307 unit / 50 server / 15 doctests.

## Review request

`.git-exclude/review-request/020-test-and-docs-follow-ups/01-mutation-follow-up-tests.md`:
the commit; the six tests with results; **each break check's output**; the
traceability rows; the gates and CI.
