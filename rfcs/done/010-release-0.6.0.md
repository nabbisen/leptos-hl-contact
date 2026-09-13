# RFC 010 — Release 0.6.0: removals, email syntax, delivery-error rule

**Status.** Implemented (0.6.0) — released 2026-09-13, tag `0.6.0` on commit `33fc677`, published to crates.io.  Accepted 2026-09-13.  The owner approved the 0.6.0 scope the
same day: the promised removals, P-34 and P-33, with P-32 through RFC 009.
**Tracks.** Roadmap P-37 (new), P-34, P-33.  Requirements FR-CFG-01,
FR-VAL-02, FR-OBS-02.  0.6.0 also carries RFC 009 (P-32).
**Touches.** `Cargo.toml`, `lib.rs`, `csrf.rs` (removed), `server.rs`,
`model.rs`, `delivery.rs`, `error.rs`, their tests, `tests/server/`,
`.github/workflows/ci.yml`, documentation, `CHANGELOG.md`.
**Handoffs.** [`../handoffs/010-release-0.6.0/README.md`](../handoffs/010-release-0.6.0/README.md)

## Summary

0.6.0 is a minor release, so it is where breaking changes go.  This RFC
tracks three small pieces whose design fits on a page:
- **The removals.**  Remove the 0.4 names, as the 0.5.0 CHANGELOG
  promised: "Every old name still works, warns, and is removed in the next
  minor."
- **Email.**  Reject email addresses no public contact could use.
- **Error text.**  Tell delivery implementers that their error text is
  logged.

RFC 009 carries the one piece that needs owner decisions.

## D1 — Remove the deprecated 0.4 names (P-37)

Removed:

| 0.4 name | Where | Replacement since 0.5.0 |
|----------|-------|--------------------------|
| feature `csrf` | `Cargo.toml` | `form-token` |
| module `csrf` with `CsrfConfig`, `CsrfToken`, `CsrfConfigContext`, `generate_csrf_token`, `verify_csrf_token` | `src/csrf.rs` | `form_token::{FormTokenConfig, FormToken, FormTokenContext, issue_form_token, verify_form_token}` |
| server-function argument `csrf_token` | `submit_contact` | `form_token` |

**Kept.**  "CSRF" as a *concept*.  The documentation's statement that
Origin validation is the CSRF control stays, and so do the old
`/security/csrf.html` redirect in `docs/book.toml` and historical records
(CHANGELOG, RFCs, roadmap).

**Effect.**  A page rendered by 0.4, which posts only `csrf_token`, is now
refused as `token_invalid`.  A 0.5 page posts `form_token` and is
unaffected.  The DOM contract does not change.

## D2 — Email addresses a contact form can reply to (P-34)

After trimming, `email` is accepted only when all of these hold:

| Rule | Rejects | Field code |
|------|---------|------------|
| the existing syntax check (`validator` `email`) | `abc`, `a @x.cz` | `Format` |
| at most **254 characters** (the SMTP path limit; the form's `maxlength` already applies it) | a 255-character address | `Length { min: 0, max: 254 }` |
| the domain is not an address literal | `a@[127.0.0.1]`, `a@[2001:db8::1]` | `Format` |
| the domain has at least two labels, none empty | `abc@bar`, `a@example.`, `a@.com`, `a@example..com` | `Format` |

- **Labels.**  No new code and no new label.  `Format` renders
  `format_email`; `Length` renders `length`.
- **Internationalised domains.**  They stay accepted, since the check
  counts labels, not characters.
- **Order.**  When several rules fail, the length code wins, so a visitor
  who pasted something very long is told why.

**Requirement.**  FR-VAL-02 becomes: "Syntactically valid email address after
trimming, at most 254 characters, whose domain is a name of at least two
labels; address literals are rejected."

## D3 — Delivery error text is logged (P-33)

`submit_contact` logs a delivery error's `Display` text at `error`
(FR-OBS-03) and never sends it to the client.  The rule for implementers,
in the `ContactDelivery` and `ContactDeliveryError` rustdoc and the delivery
guide:

> **Errors.**  The text of a returned `ContactDeliveryError` is written to
> the server log for operators.  Put the category and the transport detail
> in it — status codes, the relay's reply.  Never put the submission in it:
> no name, email address, subject, message, token or credential.  This is
> the same rule the crate follows for its own log events (FR-OBS-02).

External Design §4.4.1 (trait contract) and §4.6 (observability interface)
gain the same rule.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Keep the aliases one more minor | Breaks a published promise in the other direction, and the aliases were only ever for 0.4 pages still open in a browser |
| DNS checks for email domains | Separate, opt-in, lower priority (P-35) |
| A new `email_domain` code and label | A second sentence for the same visitor action — fix the address; `format_email` already says it |
| Leave the error-text rule implicit | The planned HTTP adapters (P-22) make leaking the address into an error string an easy mistake |

## Compatibility

Breaking, with CHANGELOG `### Removed` and `### Changed` entries and a
migration table:
- **Removed names.**  The `csrf` names (D1) are gone.
- **Newly refused addresses.**  `user@localhost`, `user@[127.0.0.1]`, and
  addresses over 254 characters are refused.  Such addresses cannot be
  replied to from a public mailbox.

## Security considerations

- **D1.**  Removes an unused parsing path; no control changes.
- **D2.**  Tightens input; an address literal can no longer become a
  `Reply-To` target.
- **D3.**  Strengthens FR-OBS-02 for third-party code.

## Testing

Each handoff states its tests and a deliberate break.  The traceability
table's rows move with them (the RFC 008 "keeping it true" rule).

## Release implications

**0.6.0**, with RFC 009.  Before the release candidate, run the mutation
pass (release process step 8).
