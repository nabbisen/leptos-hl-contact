# Handoffs — RFC 001, Milestone M1 "green baseline"

Companion execution documents for [RFC 001](../../accepted/001-m1-green-baseline.md).
Their state is that of the RFC.  Governing documents, in order of
authority: the [Requirements Specification](../../../docs/src/development/requirements.md),
the [External Design](../../../docs/src/development/external-design.md),
RFC 001, then these handoffs.  A handoff never overrides the RFC; if one
seems to, stop and report.

## Units and order

| # | Handoff | Roadmap | Depends on | Size |
|---|---------|---------|------------|------|
| 01 | [CI gates green](./01-ci-gates.md) | P-01 | — | small |
| 02 | [Examples start; CI checks them](./02-examples-route-and-ci.md) | P-09, P-05 (example versions) | 01 | small |
| 03 | [Per-field errors render](./03-field-error-rendering.md) | P-02 | 01, 02 (for evidence) | medium |
| 04 | [Length units and ceiling](./04-length-units-and-ceiling.md) | P-04, P-07 | 01 | medium |
| 05 | [Records and rustdoc](./05-records-and-rustdoc.md) | P-05, P-03 remainder | 01–04 | small |

Do them in this order.  Each is one review request; do not combine two in
one request.  Handoffs 03 and 04 touch `server.rs`; finish 03 before
starting 04 to avoid a conflicting diff.

## Rules that apply to every handoff

**Before starting.**  Read the handoff, the sections of Requirements and
External Design it cites, and the code it names.  If anything is
ambiguous, contradictory, or would require a public API or behaviour
change that the handoff does not authorise, stop and file a clarification
request instead of guessing.

**While working.**

- Stay inside the change scope.  No drive-by refactors, renames, or
  formatting of untouched files.
- Tests live in `src/<module>/tests.rs`, never inline.
- Do not disable, `#[ignore]`, or weaken a test to make it pass.
- No secrets in code, fixtures, or logs.  No PII in log events.
- English for code comments and documentation.
- Commit per handoff; do not push until the handoff's own gates pass
  locally.  Commit messages describe the change; no attribution lines.

**Changelog.**  Each handoff adds its own lines under `CHANGELOG.md`
`[Unreleased]` when it lands; the release-readiness pass checks the
section is complete.

**Gates** (all must pass before a review request; a documentation-only
unit done out of order by owner direction is accountable for `cargo doc`
and must show the other three unchanged from the baseline):

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```

CI runs on rustc 1.91 while a developer machine may be newer.  Run the
gates on the newest stable you have; if `rustup toolchain install 1.91`
is possible, run clippy on that too and report both.

## Review request format

Write one Markdown file per handoff at
`.git-exclude/review-request/001-m1/<NN>-<slug>.md` and tell the owner
and architect its path.  Contents, in this order:

1. Handoff id and title
2. Implementation summary (a paragraph)
3. Requirements addressed (IDs)
4. Changed files (list)
5. Important implementation decisions
6. Differences from the handoff or RFC, if any, with reasons
7. Tests added or changed (names) and how they map to acceptance criteria
8. Gate output: paste the tail of each of the four commands, plus any
   handoff-specific evidence commands, verbatim
9. Unresolved issues and known limitations
10. Where you want focused review

Do not summarise evidence; paste it.  A review request without pasted
gate output is returned unread.
