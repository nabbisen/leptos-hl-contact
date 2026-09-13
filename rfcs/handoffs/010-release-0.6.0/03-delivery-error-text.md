# Handoff 010-03 — Delivery error text is logged

**RFC.** [RFC 010](../../done/010-release-0.6.0.md) D3
**Roadmap.** P-33
**Requirements.** FR-OBS-02, FR-OBS-03, FR-DEL-07
**Depends on.** Handoff 01 merged.
**Code changes.** Documentation comments only.

## Goal

Anyone writing a delivery backend knows that their error text goes to the
server log, what belongs in it, and what must never be in it.

## Change scope

Use RFC 010 D3's rule in each place below.  Adapt the wording to its
surroundings, but keep all three parts: it is logged; put category and
transport detail in it; never the submission.

| File | Change |
|------|--------|
| `crates/leptos-hl-contact/src/delivery.rs` | the `ContactDelivery` rustdoc gains an `# Errors` section with the rule |
| `crates/leptos-hl-contact/src/error.rs` | `ContactDeliveryError`'s rustdoc ("Keep these on the server — log them …") gains the rule; each variant's doc stays |
| `docs/src/guides/delivery-backends.md` | "Writing your own backend" → the contract list: replace "Do not log the visitor's name, email, or message." with the rule, covering both log events and the returned error's text |
| `docs/src/development/external-design.md` | §4.4.1 Trait contract: the rule as one contract line.  §4.6 Observability interface: one sentence saying a delivery error's text is logged verbatim, so implementations keep it free of submission data |

**Out of scope.**  Leave the contract line "The call is not time-limited
by the crate" as it is.  RFC 009 replaces it.

### CHANGELOG

Under `[Unreleased]` → `### Documentation`: one line.

## Acceptance

1. `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps` passes,
   and the `# Errors` section renders on `ContactDelivery`.  Paste the
   rendered text from `target/doc`, or the source lines.
2. `mdbook build docs` passes with no warnings.
3. **Code.**  No change outside documentation comments.  Show
   `git diff --stat`, and a `git diff` restricted to lines that are not
   comments, which is empty for `src/`.
4. **Traceability.**  The FR-DEL-07 row's last column mentions the
   documented contract.

No deliberate break: nothing here is executable.

## Review request

`.git-exclude/review-request/010-release-0.6.0/03-delivery-error-text.md`
