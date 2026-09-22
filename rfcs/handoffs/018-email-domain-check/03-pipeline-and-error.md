# Handoff 018-03 — The pipeline step, and what the visitor is told

**RFC.** [RFC 018](../../done/018-email-domain-check.md) D2, D3
**Roadmap.** P-35
**Depends on.** 02 (approved)

## Goal

The check runs where it belongs, and a visitor whose domain cannot receive
mail is told **that**, not "enter a valid email address".

## Change scope

### 1. The error the visitor sees (D3, owner decision: a distinct code)

- **`FieldErrorCode`** gains a variant, for example `EmailDomain`, whose
  JSON `kind` is `email_domain`.
- **`ContactErrorLabels`** gains the matching field with an English default:
  something like "We could not find a mail server for that domain.  Please
  check the spelling."
  - **`#[serde(default)]`,** and the field takes its place in the struct's
    `Default`.
  - **Old payloads still parse,** and a payload without the new kind is
    unchanged.  A test pins both.
- **Both are breaking for literals:** one migration line each.

### 2. The step (D2), `src/server.rs`

- **Where:** after the honeypot, the form token and field validation, and
  **before** the server policy, the challenge, the filter and delivery.
- **Skipped entirely** when the address already failed syntax validation, or
  when no `EmailDomainCheck` is in context.
- **One lookup per submission.**
- **The outcomes:**
  - `Reject` → the new field error, under `email`, returned with any other
    field errors in the same response;
  - `Accept` → nothing;
  - `Unknown(reason)` → **accept**, and log at `warn` with the fixed reason
    and **no address and no domain**.
- **Order matters and must be tested:** a honeypot hit still returns silent
  success without a lookup; a bot cannot make the server do lookups.

### 3. Tests

**L2 (server suite), both request forms:**
- a rejecting domain → the field error under `email`, nothing delivered;
- the same submission with the check absent from context → delivered;
- SERVFAIL → delivered, with the `warn`;
- the honeypot filled → silent success, **and the stub resolver saw no
  request**;
- an address that fails syntax → refused as today, **no lookup**;
- the challenge is not reached when the domain is refused.

**Worker tests:** the lookup over the stubbed `fetch`, and a dropped
submission abandoning it.

**Logs:** a probe address and a probe domain appear in no captured line.

**Break checks, required:**
- make `Unknown` reject → the SERVFAIL test fails;
- move the step before the honeypot → the honeypot test fails.

### 4. `CHANGELOG.md` `[Unreleased]`

- **Added:** replace 02's line with the shipped behaviour, naming the
  label and that the feature is off unless configured.
- **Migration:** the new `ContactErrorLabels` field, and the new
  `FieldErrorCode` variant for anyone matching it exhaustively.

## Gates

The shared gates on both toolchains; the MSRV checks; feature combinations
as in 02; both wasm suites with counts; the examples with `--locked`; CI.

## Review request

`.git-exclude/review-request/018-email-domain-check/03-pipeline-and-error.md`:
the commit; the pipeline position with line references; the error and label
as implemented; the tests with results; both break checks; the gates and CI.
