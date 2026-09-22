# Handoff 020-03 — A flaky release gate

**RFC.** [RFC 020](../../accepted/020-test-and-docs-follow-ups.md), Amendment D4
**Roadmap.** P-45
**Requirements.** NFR-TEST-02/03

## Goal

The browser suite either passes or means something.  **Tests and test
support only.**

## The defect

- **`tests/browser/support/fetch.rs`:** the stub pushes each request's body
  from a spawned async read (`JsFuture::from(request.text())`).
- **`tests/browser/site_fields.rs`,
  `the_submitted_body_carries_the_site_fields`:** asserts
  `bodies().len() == 1` after one `settle()`.
- **Observed:** one failure in five runs on `d92d522`, at that assertion,
  in the architect's own environment; CI and the dev team's runs passed.

## Change scope

### 1. `tests/browser/support/fetch.rs`

- **A waiting accessor,** for example
  `pub async fn bodies_when(&self, count: usize) -> Vec<String>`:
  - settle, check, repeat, up to a bound (a small number of ticks — pick
    one and say why);
  - on timeout, **fail with a message naming what was captured**, so a real
    regression reads as a failure and not as a hang.
- **Keep `bodies()`** for tests that legitimately assert **absence** (no
  body recorded), and change its doc comment: it returns what has been
  recorded **so far**, which is not the same as what has been sent.

### 2. Every test that asserts on captured bodies

- Today that is `the_submitted_body_carries_the_site_fields`.  Grep for
  `bodies()` and convert each one that expects a body to the waiting
  accessor.
- **Assertions that expect no body** stay on `bodies()`, and should say in
  a comment why waiting would be wrong there.

### 3. Prove it, twice

- **The fix holds:** run the browser suite **ten times in a row** and
  report the results.  One failure is a failure.
- **The test still detects a real defect:** break the form so the body
  never carries the site fields (drop one `fields[…]` input's name), and
  show the converted test failing with a message that names what was
  captured.  Restore.

### 4. Nothing else

- **No production code.**  If the fix seems to need any, stop and report.
- **No CHANGELOG entry:** tests only.

## Gates

The shared gates on both toolchains; the MSRV checks; the worker suite;
the examples with `--locked`; CI.  **Report the browser suite's ten runs.**

## Review request

`.git-exclude/review-request/020-test-and-docs-follow-ups/03-flaky-browser-gate.md`:
the commit; the helper's shape and its bound, with the reason; every
converted test; the ten runs; the break check; the gates and CI.
