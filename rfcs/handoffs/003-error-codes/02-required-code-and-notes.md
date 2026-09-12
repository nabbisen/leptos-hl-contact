# Handoff 02 — `Required` for empty values, and three notes

**RFC.** [003](../../accepted/003-error-codes.md), D2 (amended by the
handoff 01 review).
**Roadmap.** P-29.  **Requirements.** FR-I18N-02, FR-SUB-06, FR-UI-08.
**Depends on.** Handoff 01, approved 2026-09-13.

## Purpose

An empty required field should say it is required, not that it must be
between 1 and 80 characters.  Plus three small records the handoff 01
review agreed to.

## Background

`validator` reports an empty required string as `length` with `min: 1`,
so `field_error_code` maps it to `Length { min: 1, max }` and the visitor
reads a range message for a box they simply left blank.  Handoff 01 was
right to implement D2 as written; D2 is amended here because the result
does not read as finished.

`Required` is currently emitted only by `ContactServerPolicy` for
`require_subject`.

## Change scope

- `crates/leptos-hl-contact/src/model.rs` (the `validate_fields` mapping)
  and `model/tests.rs`
- `CHANGELOG.md`
- `docs/src/development/testing.md`
- `docs/src/guides/localization.md` if its wording implies a blank field
  renders as a length message

## Explicit non-change scope

The wire format, the code set, the label set, `ContactErrorLabels`, the
component, the policy, the examples.  No new code variant: `Required`
already exists.

## Required implementation

1. **Mapping.**  In `validate_fields`, when a `length` violation is
   reported and the field's value is empty after trimming, emit
   `FieldErrorCode::Required` instead of `Length`.  Take the emptiness
   from the input itself, not from the validator's params, so the rule
   reads as "the box was blank".  Apply it to `name` and `message`, the
   two fields with `min = 1`; a blank `subject` is absent rather than
   empty and is already handled by the policy.
2. **Comment.**  One line saying why: `validator` cannot distinguish
   "blank" from "too short", so the crate does, because the two need
   different sentences.
3. **CHANGELOG.**  Under the existing `[Unreleased]`:
   - *Fixed*: an empty required field now renders the `required` label
     rather than the length label.
   - *Changed*, in the RFC 003 migration note: an integrator who
     translated only `ContactFormLabels::error` and not the new `errors`
     block will see English error text until they translate it; `error`
     is now reached only for unrecognised payloads.
4. **`testing.md`.**  One sentence under the browser-testing rules: after
   editing an example, rebuild with `cargo leptos build`, not
   `cargo build`, or the server's shell and the bundle disagree — the
   symptom is a request for `<name>_bg.wasm`, a WebAssembly compile error
   in the console, and `SUBMIT_TYPES ["Document"]` where a `Fetch` was
   expected.

## Required tests

`model/tests.rs`:

- `empty_name_yields_required_code` — replaces
  `empty_name_yields_length_code`; a blank name yields `Required`.
- `empty_message_yields_required_code`.
- `over_long_name_still_yields_length_code` — the range message survives
  for a value that is actually too long (keep the existing test, renamed
  if clearer).
- `whitespace_only_name_yields_required_code` — `from_raw` trims, so
  `"   "` is blank.

## Acceptance criteria

- Tests pass; gates green on both toolchains and both feature sets; CI
  green.
- On the running example, a submit with an empty name and a 200-character
  name produce, respectively, the `required` text and the `length` text
  with the numbers substituted.  Paste both.
- With a Japanese label set, the blank case reads as the translated
  `required` string.  A `curl` transcript of the no-JS path is enough;
  no screenshot needed.

## Prohibited shortcuts

A second code variant; special-casing in the component or in
`field_text`; changing what `validator` is asked to check.

## Compatibility and security constraints

Wire-compatible: `Required` is an existing variant that any client built
on handoff 01 already renders.  No security effect.

## Known risks

None significant.  If `validator` ever reports a distinct `required`
code, the branch becomes redundant rather than wrong.

## Required evidence

Gate output, test names and results, the two rendered messages, the
Japanese transcript, CI URL.
