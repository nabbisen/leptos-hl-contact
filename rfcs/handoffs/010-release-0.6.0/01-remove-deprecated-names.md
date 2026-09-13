# Handoff 010-01 — Remove the deprecated 0.4 names

**RFC.** [RFC 010](../../done/010-release-0.6.0.md) D1
**Roadmap.** P-37
**Requirements.** FR-CFG-01, NFR-COMPAT-04

## Goal

The 0.4 names stop existing, as the 0.5.0 CHANGELOG promised.  After this
handoff:
- **Sources.**  No source file, test, CI step or current-behaviour document
  names `csrf` as a feature, module, type, function or field.
- **Concept.**  "CSRF" as a concept stays.

## Change scope

### Code

| File | Change |
|------|--------|
| `crates/leptos-hl-contact/Cargo.toml` | remove `csrf = ["form-token"]` and its comment |
| `src/lib.rs` | remove `pub mod csrf;` and the `csrf` row of the feature table |
| `src/csrf.rs`, `src/csrf/tests.rs` | delete |
| `src/server.rs` | remove the `csrf_token` argument, its row in the argument table, the `.or(csrf_token.as_deref())` fallback, and `csrf_token` from the `#[cfg(not(feature = "form-token"))]` binding |
| `src/security.rs` | line 5 comment: say `form-token` |
| `src/server/tests.rs` | remove or convert each test that exercises `csrf_token` or the `csrf` names; list each in the review request with what you did |

`src/axum_helpers/tests.rs:101` uses "CSRF" as a concept.  Leave it.

### Tests

- **`tests/server/form_token.rs`.**  Delete
  `the_0_4_csrf_token_field_is_accepted`, which asks for its own removal in
  0.6.0.
- **Its replacement.**  Add `a_0_4_csrf_token_field_is_no_longer_accepted`.
  It posts a valid token under `csrf_token` and no `form_token`, in both
  request forms, and asserts `token_invalid` and zero deliveries.
  - **Verify first.**  Check how `server_fn` treats the now-unknown field.
    If it rejects the request before `submit_contact` runs, the test
    asserts what actually happens.
  - **Report it.**  The review request states which, with the response.
  - Cite FR-VAL-06 and NFR-COMPAT-04.

### CI

`.github/workflows/ci.yml` lines 25–27:
- **Rename.**  The step becomes `clippy (ssr without form-token)`.
- **Comment.**  Reword it to say `--all-features` always turns `form-token`
  on.
- **Command.**  Unchanged.

### Documentation (current behaviour only)

| File | Change |
|------|--------|
| `docs/src/reference/feature-flags.md` | remove the `csrf` row |
| `docs/src/reference/api.md` | remove every 0.4 name |
| `docs/src/security/form-token.md` | remove alias instructions.  Keep one short "Upgrading from 0.4" note: the 0.4 names were removed in 0.6.0; see the CHANGELOG |
| `docs/src/security/hardening.md:10` | reword "`csrf` case" to the form token |
| `docs/src/development/architecture.md` | lines 61, 115, 131: remove `csrf`.  Line 15: "CSRF" → "form token" |
| `docs/src/development/external-design.md` | remove the 0.4 names from current-behaviour sections; the threat model's CSRF wording stays |
| `docs/src/development/testing.md` | remove the `csrf/tests.rs` organisation row; update the NFR-COMPAT-04 and FR-VAL-06 rows |
| `docs/src/development/requirements.md` | FR-CFG-01: remove "; `csrf` is a deprecated alias" and set the status to "Met (0.6.0)".  Add a change-history row "Draft 14 — RFC 010 D1: `csrf` alias removed"; bump the document status line |

Do **not** edit: `CHANGELOG.md` history sections, `rfcs/`, `ROADMAP.md`,
`docs/book.toml`'s old-URL redirect, and README's "the CSRF control".

### CHANGELOG

Under `[Unreleased]`, add a `### Removed` entry listing the removed names.
Add a migration table, 0.4 name → name to use, copied from the 0.5.0
Migration table.

## Acceptance

1. **Grep.**  `git grep -n -i -E 'csrf_token|CsrfConfig|CsrfToken|csrf_config|generate_csrf|verify_csrf|mod csrf|"csrf"|\bcsrf =|feature.*\bcsrf\b'`
   must show nothing outside `CHANGELOG.md`, `rfcs/`, `ROADMAP.md`,
   `docs/book.toml`, and the one "Upgrading from 0.4" note.  Paste the
   output.
2. **The new server test** passes in both forms.
3. **Deliberate break.**  Restore only the `.or(csrf_token…)` fallback, with
   the argument re-added, and show the new test fail.  Then restore.
4. **Gates.**  All pass, plus the server and browser suites.  Report the unit
   test count before and after, with each removed test named.
5. **Traceability.**  The table has no reference to a removed test.

## Review request

`.git-exclude/review-request/010-release-0.6.0/01-remove-deprecated-names.md`
