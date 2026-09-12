# Handoff 05 — Records and rustdoc

**RFC.** [001](../../accepted/001-m1-green-baseline.md).
**Roadmap.** P-05, P-03 (code remainder).  **Requirements.** NFR-REL-02, FR-CFG-01, NFR-DOC-01.
**Depends on.** Handoffs 01–04 (this one closes the changelog for them).

## Purpose

Make the release records and the crate-level rustdoc truthful.

## Background

- `CHANGELOG.md` labels 0.2.0, 0.2.2, 0.3.0, 0.3.1, 0.3.2, 0.3.3 as
  "Unreleased" although tags exist for each.  0.2.1 and 0.2.3 are tagged
  but have no entry.
- `src/lib.rs` crate docs: the feature table omits `csrf`; the security
  link points to `docs/src/security.md`, which no longer exists; the
  Quick start paragraph points to `examples/axum-basic`, which is the
  local-dev example.
- `src/security.rs` header comment says the module "does not export
  runtime functionality in the MVP" and promises "future CSRF helpers".
- `src/delivery.rs` header comment says `delivery/mod.rs`.
- `src/csrf.rs` rustdoc calls `CsrfToken` "a single-use CSRF token" (line
  100) and the module "CSRF token helper".  It is neither single-use nor a
  CSRF control on its own (External Design §5.4).  Reported by the dev
  team 2026-09-12; confirmed.
- `src/axum_helpers.rs` rustdoc examples (lines 45 and 82) show the Axum
  0.7 route `"/api/*fn_name"`; they are `ignore`-fenced so CI cannot
  catch them.  Same defect class as P-09.  Reported by the dev team
  2026-09-12; confirmed.

## Change scope

`CHANGELOG.md`; comments and rustdoc in `src/lib.rs`, `src/security.rs`,
`src/delivery.rs`, `src/csrf.rs`, `src/axum_helpers.rs`.  No code.

## Explicit non-change scope

No source code, no tests, no documentation pages in `docs/src`.

## Required implementation

1. **CHANGELOG.**
   - Every tagged version carries its tag date (`git tag --format='%(refname:short) %(creatordate:short)'`);
     replace "— Unreleased" labels and correct any existing date that
     disagrees with the tag.
   - Add entries for `0.2.1` and `0.2.3` from their tag diffs
     (`git diff 0.2.0..0.2.1 --stat`, etc.); one or two lines each is
     enough.
   - Add a top section `## [Unreleased]` collecting the M1 changes from
     handoffs 01–04 under *Fixed* / *Added* / *Changed*, and the docs
     restructure of 2026-09-12 under *Documentation*.  Do not assign a
     version number; the owner decides.
2. **`lib.rs` crate docs.**
   - Feature table: add `csrf` row: "Stateless HMAC-SHA256 anti-automation
     token; `submit_contact` requires `CsrfConfigContext` (fail-closed)."
   - Security paragraph link →
     `https://github.com/nabbisen/leptos-hl-contact/blob/main/docs/src/security/README.md`.
   - Quick start paragraph: point to `examples/axum-with-security` for
     production wiring and `examples/axum-basic` for local development.
3. **`security.rs` header.**  Replace the three-point list with one
   sentence: the module holds defence-in-depth helpers shared by other
   modules; CSRF lives in `csrf`.
4. **`delivery.rs` header.**  `delivery/mod.rs` → `delivery.rs`.
5. **`csrf.rs` rustdoc.**  Do not rename anything (RFC 004 does that in
   0.5.0).  Change only the wording: line 100 becomes "A signed,
   time-limited token value, ready to embed in an HTML form.  It is valid
   until it expires and is not bound to the visitor's browser; Origin
   validation is the CSRF control.  See the security documentation."
   The module header line 1 and the `CsrfConfig` doc line 43 say
   "anti-automation token helper (feature `csrf`)".  Log strings and
   identifiers stay as they are.
6. **`axum_helpers.rs` rustdoc.**  Both examples: `"/api/*fn_name"` →
   `"/api/{*fn_name}"`.  Then run
   `grep -rn '"/api/\*fn_name"' crates/ docs/ examples/ README.md`; the only
   permitted matches are the two example `main.rs` files, which handoff 02
   owns.  Prose that names the old form in order to warn against it is
   correct and stays.

## Required tests

None.  `cargo doc --all-features --no-deps` must produce no warnings.

## Acceptance criteria

- No "Unreleased" label on a tagged version; every tag `0.1.0`–`0.3.3` has
  an entry with a date.
- `cargo doc` clean; the rendered crate page lists six features.
- `git grep -n "docs/src/security.md" -- '*.rs'` returns nothing.
- `git grep -n 'single-use' -- '*.rs'` returns nothing;
  `grep -rn '"/api/\*fn_name"' crates/ docs/ examples/ README.md` returns
  at most the two example files (empty once handoff 02 has landed).

## Prohibited shortcuts

Inventing dates; deleting historical entries; rewriting past entries
beyond the date label, the two missing versions, and corrections of
entries proven wrong by the tag diffs (state the correction in one line).

## Module boundaries, compatibility, security

Documentation only.  None affected.

## Known risks

Tag dates and commit dates may differ if tags were created late; use the
tag's own date and, where it is clearly wrong, the tagged commit's author
date, and say which you used.

## Required evidence

`cargo doc` tail; `git tag --format='%(refname:short) %(creatordate:short)'`;
the `git grep` result.
