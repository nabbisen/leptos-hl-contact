# RFC 001 — Milestone M1: green baseline

**Status.** Implemented (0.3.4) — released 2026-09-12, tag `0.3.4` on
commit `4c3575a`, published to crates.io.  Accepted 2026-09-12; all five
handoffs approved the same day.
**Tracks.** Roadmap items P-01, P-02, P-03 (code remainder), P-04, P-05,
P-07, P-09.
**Touches.** `crates/leptos-hl-contact/src/{components,error,config,model,
server,security,lib}.rs` and their test modules; `examples/*`;
`.github/workflows/ci.yml`; `CHANGELOG.md`; small documentation edits.
**Handoffs.** [`../handoffs/001-m1-green-baseline/README.md`](../handoffs/001-m1-green-baseline/README.md)

## Summary

M1 is a patch-level release that restores trust in the project's own
gates and makes the crate behave as its documentation already says.  It
introduces no new feature and changes no public API.  It is tracked as an
RFC so that its handoffs have a lifecycle anchor per RFC-000; the design
decisions inside are small and are recorded here rather than in separate
RFCs.

## Motivation

The architect's baseline review of `0.3.3` found that CI gates fail on
`main`, that per-field validation errors never render in the browser,
that both bundled examples panic at startup, and that several documented
invariants (character-based limits, the 4 000-character ceiling) are not
enforced.  Each defect contradicts a MUST-level requirement in the
[Requirements Specification](../../docs/src/development/requirements.md).

## Goals

- All four CI gates green on `main` (NFR-TEST-01, NFR-DOC-02).
- Per-field errors rendered beside their fields (FR-UI-08, FR-SUB-06).
- Length units consistent and the ceiling enforced (FR-VAL-07, FR-VAL-08).
- Both examples start and serve the form; CI compiles them (P-09).
- Records accurate: CHANGELOG dates, example versions, rustdoc feature
  table (NFR-REL-02, FR-CFG-01).

## Non-goals

Anything in M2: form state preservation, token survival across
re-render, anti-forgery redesign, no-JS success feedback, error codes,
integration test suite.  If a handoff cannot be completed without one of
these, stop and report.

## Design decisions recorded here

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | The component recognises a field-error payload by matching the `ServerFnError::Args` variant and parsing its inner string; `ContactFieldErrors::from_error_str` additionally tolerates leading text before the sentinel | Variant matching is exact in both WASM and no-JS paths (the URL-encoded error preserves the variant); the tolerant parser is defence against display wrapping and stays backward compatible |
| D2 | A single `MESSAGE_MAX_LEN: usize = 4000` constant in `model` is the ceiling; the validator attribute, `ContactFormOptions`, and `ContactServerPolicy` all derive from it, and values above it are clamped | One source of truth; clamping is silent because the validator enforces the ceiling regardless, so this is not a security fallback |
| D3 | `ContactServerPolicy` gains a pure `check(&ContactInput) -> ContactFieldErrors` method called from `submit_contact` | Makes policy testable without Leptos context; prepares for the M2 integration tests |
| D4 | `sanitize_header_value` keeps replacing each CR and LF with one space; the rustdoc example is corrected | Behaviour is defence-in-depth only; changing it gains nothing |
| D5 | Examples move to Axum 0.8 wildcard syntax and CI gains a job that `cargo check`s each example | The examples are the documentation's executable proof; they must at least compile in CI |

## Compatibility

Patch release.  Public additions only: `MESSAGE_MAX_LEN`,
`ContactFieldErrors::from_server_fn_error`, `ContactServerPolicy::check`.
No removals, no signature changes, no DOM contract changes.

## Security considerations

No new data flow.  D1 changes only client-side parsing.  D2 and D3 tighten
enforcement.  Threat model unchanged.

## Testing

Each handoff lists its tests.  Gate: `cargo fmt --all --check`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all-features`, `cargo doc --all-features --no-deps`, plus
the new examples job.

## Acceptance criteria

All handoffs under `../handoffs/001-m1-green-baseline/` reviewed
**Approved** by the architect; gates green; release-readiness report
submitted to the owner proposing `0.3.4`.

## Release implications

Proposed `0.3.4`.  Version number and timing are the owner's decision.
