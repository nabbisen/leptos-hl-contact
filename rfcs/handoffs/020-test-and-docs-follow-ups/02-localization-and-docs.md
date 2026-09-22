# Handoff 020-02 — Translations as contributed examples, and two documentation lines

**RFC.** [RFC 020](../../accepted/020-test-and-docs-follow-ups.md) D2, D3
**Roadmap.** P-20 (reshaped), and the reflerd.com letter of 2026-09-22
**Requirements.** FR-UI-02, FR-I18N-02

## Goal

Make it easy for a site to translate the form, and easy for someone to
contribute the next language, **without the crate shipping translations it
cannot verify**.

## Change scope

### 1. `docs/src/guides/localization.md`

- **Keep the existing complete example** (Japanese) exactly as it is.
- **Add a short section, "Contributing a language":**
  - **What a contribution is:** one complete set — every field of
    `ContactFormLabels` and `ContactErrorLabels`, no gaps — in the
    contributor's own language, as a copyable block on this page.
  - **Why complete:** a partial set leaves a visitor reading two languages
    in one form.
  - **Why the page and not the crate:** the crate ships English defaults
    and every visible string is the site's to set; a translation here
    carries no version coupling, and a new error code cannot silently leave
    a shipped language half-translated.
  - **What we check:** that the set is complete and the placeholders
    (`{min}`, `{max}`) are intact.  **We do not vouch for the wording** —
    the contributor's language is theirs.
- **Say plainly, near the top:** the crate ships English defaults, and
  nothing on the page is a guarantee about a language none of us speaks.

### 2. `docs/src/guides/customization.md`, site-defined fields

One line beside the `<select>` sentence: **keep a choice label short enough
to read inside a phone's select box.**  A native select shows only what
fits; the label is what a visitor sees, the key is what delivery records.
(An integrator's 320 px phone truncated "Foundation design for new
projects"; they shortened it, and nothing in the crate needed changing.)

### 3. Nothing else

- **No new language written by us.**
- **No code, no feature, no label change.**

## Gates

`mdbook build docs`: exit 0, no warnings, and every anchor you add or link
checked in the built HTML.  The shared gates need not be re-run for a
docs-only change, but say that `git status` shows only `docs/`.

## Review request

`.git-exclude/review-request/020-test-and-docs-follow-ups/02-localization-and-docs.md`:
the commit; the new section's text; the one-line addition; the mdbook
result.
