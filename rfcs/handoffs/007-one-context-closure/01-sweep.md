# Handoff 01 — One closure: docs, examples, rustdoc

**RFC.** [007](../../done/007-one-context-closure.md) D1, D3.
**Requirements.** FR-CFG-02, NFR-DOC-01.

## Purpose

Remove the "two context sites" instruction everywhere and make both
examples follow the one-closure pattern.

## Change scope

- `examples/axum-basic/src/main.rs`, `examples/axum-with-security/src/main.rs`
- `docs/src/getting-started/quick-start.md`, `getting-started/production-checklist.md`,
  `guides/axum-integration.md`, `guides/accessibility.md`, `security/README.md`,
  `security/csrf.md`, `help/faq.md`, `help/troubleshooting.md`,
  `development/external-design.md` §2.2 and §2.1 step 4
- rustdoc in `src/axum_helpers.rs`, `src/csrf.rs`, `src/config.rs`
  (`ContactServerPolicy`, `ContactSuccessRedirect`) and `src/delivery.rs`
  wherever "both" closures or "server-function handler closure" appear
- `CHANGELOG.md` Unreleased (Documentation)

## Explicit non-change scope

No crate logic; no feature or type changes; the security layers and their
order; RFC 004 handoff text (the architect amended it).

## Required implementation

1. **Examples.**  Delete the `.route("/api/{*fn_name}", …)` block from
   both.  Move every `provide_context` into the single closure passed to
   `leptos_routes_with_context`.  In `axum-with-security`, construct
   `success_redirect("/thanks")` **once before the router** and clone it
   into the closure, so a bad path fails at boot (RFC 002 handoff 03
   review, §10.3).  Keep `generate_csrf_token` in the closure with a
   one-line comment that it is unused on server-function requests.
2. **Docs.**  Replace the two-sites explanation with the one-closure rule
   and a short "why": `leptos_routes_with_context` registers server
   functions itself.  `axum-integration.md` keeps one "Advanced" paragraph
   for the exclusions variant (RFC 007 D3).  The context table in
   `external-design.md` §2.2 collapses its two site columns into one
   "provided in the context closure" column; the sentence "The two-site
   rule is a Leptos constraint…" is replaced by the correct statement.
   `security/csrf.md` step 4 becomes one snippet.  `troubleshooting.md`
   "not configured" entries say "the context closure".
3. **Production Checklist.**  Add a row: "Success page configured
   (`success_redirect`) if visitors without JavaScript must see a
   confirmation" linking to Customization.
4. **Accessibility guide.**  One sentence: with a success page configured
   the confirmation is a navigation to a page with an `<h1>`, not a live
   region; both are accessible, and the page should say what happened in
   its first heading.
5. **Rustdoc.**  Same wording change in the files listed; `cargo doc`
   clean.
6. **Sweep check.**  `grep -rn 'fn_name\|both context\|two context\|two-site\|server-function handler closure' docs/src crates examples README.md`
   returns only the Advanced paragraph and this RFC's own text.

## Required tests

None in Rust.  Evidence is the example behaving identically.

## Acceptance criteria

- The grep above is clean apart from the permitted matches.
- On the rebuilt `axum-with-security` (clean bundle): the RFC 002
  handoff 02 probe (two invalid submits, then valid) shows the token
  surviving and `/thanks` on success; the no-JS curl shows `302 /thanks`
  for valid and `__err` for invalid data; `axum-basic` still serves the
  form.  Paste the records.
- Gates and CI green; `mdbook build` clean.

## Prohibited shortcuts

Keeping the manual route "for safety"; documenting both patterns as
equal; touching RFC 004 handoffs.

## Known risks

None significant; this removes code and duplicated instructions.

## Required evidence

The grep output, the probe records, the curl transcripts, gate output,
CI URL.
