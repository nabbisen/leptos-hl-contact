# Handoff 01 — CI gates green

**RFC.** [001](../../accepted/001-m1-green-baseline.md), decision D4.
**Roadmap.** P-01.  **Requirements.** NFR-TEST-01, NFR-DOC-02.

## Purpose

Make `main` pass its own four gates so that every later review can trust
them.

## Background

Observed on 2026-09-12 at commit `8d29d5a` with rustc 1.98.1:

- `cargo fmt --all --check` reports diffs in 25 locations across most
  source and test files.
- `cargo clippy --all-targets --all-features -- -D warnings` fails with
  four errors:
  - `field_reassign_with_default`, `crates/leptos-hl-contact/src/server.rs`
    around lines 126–132 (two occurrences, the policy block).
  - unused import `validator::Validate`, `src/model/tests.rs:4`.
  - unused variable `token`, `src/csrf/tests.rs:58`.
- `cargo test --all-features` fails one doctest:
  `security::sanitize_header_value` expects `"Hello Injected: header"`
  but the function yields `"Hello  Injected: header"` (CR and LF each
  become a space).
- The 40 unit tests pass.

## Change scope

- Formatting of all files under `crates/leptos-hl-contact/src/` as
  produced by `cargo fmt`.
- The four clippy sites named above.
- The doctest in `src/security.rs`.

## Explicit non-change scope

- No behaviour change anywhere.  `sanitize_header_value` keeps its
  current output (D4).
- No changes to `Cargo.toml`, CI workflow, examples, or documentation
  other than the rustdoc example named below.
- Do not add `#[allow(...)]` attributes to silence lints.

## Required implementation

1. **Formatting.**  Run `cargo fmt --all` once and commit the result as
   its own commit titled `fmt: apply rustfmt` before any other change, so
   the logic diff in the next commit is readable.
2. **`server.rs` policy block.**  Replace the two
   `let mut errs = ContactFieldErrors::default(); errs.<field> = Some(..)`
   sequences with struct literals:
   `ContactFieldErrors { subject: Some("Subject is required.".into()), ..Default::default() }`
   and the equivalent for `message`.  You may drop the redundant inner
   `#[cfg(feature = "ssr")]` around that block while you are there; it
   sits inside an outer block with the same cfg.  Nothing else in
   `server.rs` changes (handoffs 03 and 04 will touch it later).
3. **`model/tests.rs`.**  Remove `use validator::Validate;`; the tests
   call `validate_input`, which is an inherent method.
4. **`csrf/tests.rs`, `expired_token_fails_verification`.**  Delete the
   unused `let token = generate_csrf_token(&config);` line and the
   comment block that explains why it is not used; keep the fabricated
   old-token assertion, which is the actual test.
5. **`security.rs` rustdoc.**  Change the example so it does not depend on
   CRLF spacing and states the rule:

   ```rust
   /// Each CR and LF is replaced by one space, so a CRLF pair becomes two
   /// spaces.
   ///
   /// let safe = sanitize_header_value("Hello\nInjected: header");
   /// assert_eq!(safe, "Hello Injected: header");
   /// assert_eq!(sanitize_header_value("a\r\nb"), "a  b");
   ```

## Required tests

No new test functions.  Existing 40 unit tests and all doctests pass.

## Required documentation updates

None beyond the rustdoc example.

## Acceptance criteria

- The four gate commands exit 0 on the newest stable toolchain available.
- `git diff 8d29d5a..HEAD -- '*.rs' ':!*/tests.rs'` shows, apart from
  whitespace, only the changes in items 2 and 5.
- Two commits: the formatting commit and the fix commit.

## Prohibited shortcuts

`#[allow]`, `rustfmt::skip`, `--no-verify`, changing the sanitiser to fit
the old doc string.

## Module boundaries

`server.rs`, `model/tests.rs`, `csrf/tests.rs`, `security.rs`; every other
file only via `cargo fmt`.

## Compatibility and security constraints

None affected.  Patch-level.

## Known risks

Clippy on rustc 1.91 (CI) may name lints that 1.98 does not, or vice
versa.  If CI fails after push on a lint that does not reproduce locally,
fix it in a follow-up commit within this handoff and paste the CI log
excerpt in the review request.

## Required evidence

Verbatim tails of the four gate commands; `git log --oneline -3`;
`git diff --stat 8d29d5a..HEAD`.
