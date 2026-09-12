# Handoff 04 — Length units and the message ceiling

**RFC.** [001](../../accepted/001-m1-green-baseline.md), decisions D2, D3.
**Roadmap.** P-04, P-07.  **Requirements.** FR-VAL-04, FR-VAL-07, FR-VAL-08, FR-SUB-07.
**External Design.** §4.3 (policy can only tighten); §6 (characters everywhere).
**Depends on.** Handoff 01; do after handoff 03 (both touch `server.rs`).

## Purpose

Count message length the same way everywhere, and make the 4 000-character
ceiling a single constant that nothing can exceed.

## Background

- `validator`'s `length` counts `chars()` (validator 0.20,
  `validation/length.rs`).  `ContactServerPolicy` enforcement in
  `server.rs` compares `input.message.len()`, which is bytes.  A 2 000-
  character Japanese message is about 6 000 bytes and is rejected by a
  policy of 4 000 even though the validator and the textarea accept it.
- `ContactFormOptions.max_message_len` and
  `ContactServerPolicy.max_message_len` document "must not exceed 4 000"
  but nothing enforces it; the literal `4000` appears in `model.rs`,
  `config.rs` (twice), and documentation.

## Change scope

- `src/model.rs`, `src/model/tests.rs`
- `src/config.rs`, `src/config/tests.rs`
- `src/server.rs` (policy block only)
- `src/components.rs` (the `maxlength` expression only)
- `src/lib.rs` (re-export)
- `docs/src/guides/customization.md`, `docs/src/reference/api.md`
- `.github/workflows/ci.yml` (added by review of handoff 02): one
  feature-combination clippy step in `check`; `RUSTFLAGS: -D warnings` on
  the `examples` job

## Explicit non-change scope

- Validation rules for `name`, `email`, `subject` unchanged.
- No change to error message texts (P-14 is M2).
- No change to the wire format.

## Required implementation

1. **Constant.**  In `model.rs`:
   `pub const MESSAGE_MAX_LEN: usize = 4000;` with rustdoc: "Hard ceiling
   for `message`, in characters (Unicode scalar values).  UI options and
   server policy are clamped to it."  Re-export from `lib.rs`.
   Use it in the validator attribute:
   `#[validate(length(min = 1, max = MESSAGE_MAX_LEN, message = "…"))]`
   (`validator_derive` 0.20 accepts an expression for `max`).
2. **Policy check as a pure method.**  In `config.rs`:

   ```rust
   impl ContactServerPolicy {
       /// Effective message limit: `max_message_len` clamped to `MESSAGE_MAX_LEN`.
       pub fn effective_max_message_len(&self) -> usize { self.max_message_len.min(MESSAGE_MAX_LEN) }

       /// Apply the policy to a normalised input.  Empty result means the
       /// input passes.
       pub fn check(&self, input: &ContactInput) -> ContactFieldErrors { … }
   }
   ```

   `check` sets `subject = Some("Subject is required.")` when
   `require_subject && input.subject.is_none()`, and
   `message = Some(format!("Message must be at most {} characters.", limit))`
   when `input.message.chars().count() > limit` where
   `limit = self.effective_max_message_len()`.  Both may be set at once.
   `Default` for the policy uses `MESSAGE_MAX_LEN`.
3. **`server.rs`.**  Replace the inline policy block with:

   ```rust
   if let Some(policy) = use_context::<ContactServerPolicy>() {
       let errs = policy.check(&input);
       if !errs.is_empty() {
           return Err(ServerFnError::Args(errs.into_server_fn_message()));
       }
   }
   ```

4. **`ContactFormOptions`.**  `Default` uses `MESSAGE_MAX_LEN`.  Add
   `pub fn effective_max_message_len(&self) -> usize` with the same clamp.
   In `components.rs` render `maxlength=options.effective_max_message_len().to_string()`.
   Update the rustdoc of both `max_message_len` fields from "must not
   exceed" to "values above `MESSAGE_MAX_LEN` are clamped".

5. **Feature-combination warning (added by review of handoff 02).**  With
   `ssr` on and `csrf` off the crate emits `unused variable: csrf_token` in
   `server.rs`.  Silence it structurally: inside the `ssr` block add
   `#[cfg(not(feature = "csrf"))] let _ = &csrf_token;` next to the
   `#[cfg(feature = "csrf")]` block, with a one-line comment.  Then add to
   the `check` job, after the existing clippy step:
   `cargo clippy --all-targets --no-default-features --features ssr,smtp-lettre,axum-helpers -- -D warnings`
   and set `env: RUSTFLAGS: -D warnings` on the `examples` job.  Both must
   be green in the run you cite.

## Required tests

`src/config/tests.rs`:

- `policy_default_matches_ceiling`
- `policy_check_passes_valid_input`
- `policy_check_requires_subject_when_set`: input with `subject: None`,
  `require_subject: true` → `subject` error set, `message` error unset.
- `policy_check_counts_characters_not_bytes`: `"あ".repeat(100)` with
  `max_message_len: 100` passes; `"あ".repeat(101)` fails.
- `policy_check_clamps_to_ceiling`: `max_message_len: 10_000` and a
  4 001-character message → `message` error; the message text names
  `4000`.
- `policy_check_reports_both_errors_at_once`.
- `options_effective_len_is_clamped`: `max_message_len: 9_999` → `4000`;
  `1_000` → `1_000`.

`src/model/tests.rs`:

- `message_ceiling_constant_is_enforced_by_validator`: `"x".repeat(MESSAGE_MAX_LEN)`
  passes, `MESSAGE_MAX_LEN + 1` fails (replaces the hard-coded 4001 test
  or sits beside it).
- `message_length_counts_characters`: `"あ".repeat(MESSAGE_MAX_LEN)`
  passes validation.

## Required documentation updates

- `guides/customization.md`: remove the blockquote about bytes; in the
  two tables say "clamped to 4 000".
- `reference/api.md`: add `MESSAGE_MAX_LEN` under `model`, and the two
  `effective_max_message_len` methods and `ContactServerPolicy::check`.
- `CHANGELOG.md` *Unreleased* → *Fixed*: policy counts characters;
  *Added*: constant and methods.  (Handoff 05 finalises the changelog.)

## Acceptance criteria

- Tests above pass; gates green.
- `grep -rn "4000\|4 000" crates/leptos-hl-contact/src --include='*.rs'`
  matches only the constant definition, its rustdoc, and test files.
- Behaviour: a policy of 100 accepts a 100-character multibyte message
  (covered by the unit test; no manual evidence needed).

## Prohibited shortcuts

Counting graphemes or UTF-16 units; keeping a second literal `4000`
anywhere; changing the validator to bytes to match the old policy.

## Module boundaries

`model` owns the constant and validation; `config` owns policy and option
semantics; `server` only orchestrates; `components` only renders.

## Compatibility constraints

Additive public API.  A policy value above 4 000, previously ineffective,
is now clamped: identical observable behaviour.  Patch-level.

## Security constraints

Clamping must be in the direction of stricter limits only.  The validator
remains the last line regardless of policy.

## Known risks

If `validator_derive` rejects the constant path in `length(max = …)`,
keep the literal in the attribute and add a test asserting
`MESSAGE_MAX_LEN == 4000` next to a comment naming the attribute; report
this in the review request.

## Required evidence

Gate outputs; test names and results; the `grep` result from the
acceptance criteria.
