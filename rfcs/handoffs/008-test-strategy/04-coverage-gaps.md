# Handoff 008-04 — Close the cheap coverage gaps

**RFC.** [RFC 008](../../done/008-test-strategy.md), D5 follow-up
**Origin.** The handoff 03 review, 2026-09-13: the traceability table found
rows that a small test or one CI step would cover.
**Depends on.** Handoffs 01–03 (done).
**Changes to `src/`.** None expected.  If a test shows the code does not do
what the requirement says, stop and report; do not fix it in this handoff.

## Why

The table in `docs/src/development/testing.md` says "review" for rows that
are cheap to automate.  A property checked only by review is checked once.
These are the rows where a test costs minutes and then guards the property
at every push.

## Tasks

### A — SSR attribute tests (L1, `src/components/tests.rs`)

Render `ContactForm` server-side, as the existing `components::` tests do,
and assert on the markup.  One test per concern, each citing its IDs.

| Test (suggested name) | Asserts | IDs |
|-----------------------|---------|-----|
| `every_input_has_a_label_for_it` | each visible control's `id` has exactly one `<label for="…">`; no control relies on `placeholder` | FR-A11Y-01 |
| `required_fields_carry_both_required_attributes` | `name`, `email`, `message` carry `required` and `aria-required="true"`; `subject` does neither by default, and does both with `require_subject` | FR-A11Y-02, FR-UI-01, FR-UI-11 |
| `the_honeypot_is_hidden_from_everyone` | the `website` input is `aria-hidden="true"` (on it or its wrapper), `tabindex="-1"`, `autocomplete="off"`, and inside the off-screen wrapper | FR-UI-10, FR-A11Y-06 |
| `maxlength_matches_the_validator` | `maxlength` is 80 on `name`, 120 on `subject`, and the effective message length on `message` — including a UI option above the ceiling being clamped | FR-VAL-07, FR-VAL-08 |

Email's `maxlength="254"` has no matching validator limit today.  Record
that in the table's last column; do not change it.

### B — `SmtpConfig` redaction (L1, `src/delivery/smtp/tests.rs`)

`debug_redacts_the_password`: a config with password `"hunter2-test"`;
`format!("{config:?}")` contains `<redacted>` and not the password.  Do the
same for `LettreSmtpDelivery` if its `Debug` prints the config.  (FR-CFG-04)

### C — Pending state (L3, `tests/browser/`)

`pending::the_submit_button_is_busy_while_sending`, with a `FetchStub` whose
`/api/submit_contact` promise never resolves.
- **After `fill_valid()`, `submit()` and `settle()`:** the submit button is
  `disabled`, has `aria-busy="true"`, and its text is `labels.sending`.
- **Before the submit:** not disabled, `aria-busy="false"`, text
  `labels.submit`.

Use custom `sending` and `submit` labels, so the test also shows they come
from the labels.  (FR-UI-05, FR-A11Y-05, FR-UI-02)

A `FetchStub` that never answers may need a new constructor.  Keep it
in `support/fetch.rs`.

### D — wasm32 with default features (CI)

In the `check` job, add the `wasm32-unknown-unknown` target to the
toolchain step.  Then add one step:

```
cargo check -p leptos-hl-contact --target wasm32-unknown-unknown
```

This builds `default = []` on wasm32, which nothing else in CI does.  It
passes locally today.  (NFR-PORT-01)

### E — The table

1. **Five new rows**, made MUST in Requirements Draft 12: NFR-COMPAT-01,
   -02, -05, NFR-TEST-04, NFR-REL-01.  Remove them from the introduction's
   "not listed" sentence; NFR-DOC-04 stays there, now as a SHOULD.  Likely
   entries:
   - **NFR-COMPAT-01:** the CI builds (all features, hydrate on wasm32);
     Islands by review.
   - **NFR-COMPAT-02:** none.  CI runs 1.91, not 1.85, so it is review until
     P-24.  Say so.
   - **NFR-COMPAT-05, NFR-REL-01:** review at release.
   - **NFR-TEST-04:** review.
2. **The rows A–D change:** FR-UI-01, FR-UI-05, FR-UI-10, FR-UI-11, FR-VAL-07,
   FR-CFG-04, FR-A11Y-01, -02, -05, -06, NFR-PORT-01.
3. **Align the NFR-SEC-01 row** with the other untested rows.

### F — Ignore local toolchain target directories

Add `/target-*/` to `.gitignore`.  A 3 GB `target-1.91/` from the 1.91 gate
runs currently shows as untracked, one `git add -A` away from a commit.

## Out of scope

- `mdbook test` in CI (NFR-DOC-03): rustdoc fences in the book need the crate
  linked with features; a separate question.
- An automated accessibility audit (FR-A11Y-09).
- MSRV 1.85 in CI: P-24.

## Acceptance

- Tests A–C exist, pass, and cite their IDs.
- **Deliberate breaks** (local, never committed): make one for A (remove
  `aria-required` from `email`) and one for C (drop `aria-busy`).  Show that
  each named test fails, then restore.
- The CI `check` job runs the new step and passes.
- Every row in E is updated; the table's counts (MUST rows, rows without a
  test) are reported before and after.
- Gates green on a clean crate, by exit code; the browser suite passes.

## Review request

`.git-exclude/review-request/008-test-strategy/04-coverage-gaps.md`:
- the commit and CI run;
- the test names;
- both break transcripts;
- the before/after counts;
- anything in A that did not match the requirement.
