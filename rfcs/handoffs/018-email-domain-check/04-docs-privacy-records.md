# Handoff 018-04 — Documentation, privacy, traceability

**RFC.** [RFC 018](../../accepted/018-email-domain-check.md) D5, D6
**Roadmap.** P-35
**Depends on.** 03 (approved)

## Change scope

### 1. The live test (D6)

An `#[ignore]`d test that queries a real resolver for the cases whose
answers are stable: a domain with a mail route, and one with a null MX.
- **Rustdoc:** how to run it, and that it is skipped by default.
- **It must not** assert anything about a domain we do not control the
  expectations of beyond those two shapes.

### 2. Documentation

| Page | Change |
|------|--------|
| `docs/src/security/` (a new page, or a section where validation is described) | **What the check is and is not:** it proves a domain can receive mail, never that a mailbox exists; the crate never sends probe mail; every failure of the lookup accepts the submission |
| the same page | **Privacy, in the voice the challenge page uses for vendors:** the **domain** leaves the server on every checked submission, the local part never does, the site chooses the resolver, and the site should say so in its privacy notice |
| `guides/customization.md` or wherever labels are listed | the new label, with its English default |
| `guides/localization.md` | the new label in the complete example, so a translated site does not ship one English sentence |
| `reference/feature-flags.md` | the new feature, and its Workers column |
| `reference/api.md` | `EmailDomainCheck`, its constructor and builder, the default timeout, and the new error code |
| `getting-started/production-checklist.md` | one row: the resolver is named, and the privacy notice mentions it |
| `guides/cloudflare-workers.md` | one line: it works on a Worker because the lookup goes over HTTPS, not UDP |
| `development/external-design.md` | the new step in the pipeline table, and the error mapping |
| `development/architecture.md` | the source layout, exactly |
| `development/testing.md` | the traceability rows for the new tests, **and the one-line follow-up carried from the RFC 020 handoff 01 review: cite `http::tests::a_64_000_byte_body_is_accepted_65_537_bytes_is_unusable` under NFR-SEC-07** |

### 3. Records

- **`requirements.md` is mine.**  List the rows you think need a status
  change; I write them at review.  FR-VAL-02's neighbours and NFR-PRIV are
  the likely ones.

## Gates

The shared gates; the MSRV checks; both wasm suites; the examples with
`--locked`; `mdbook build docs` with no warnings and every new anchor
checked; CI.  Say whether the live test was run.

## Review request

`.git-exclude/review-request/018-email-domain-check/04-docs-privacy-records.md`:
the commit; each page's change; the privacy paragraph in full; the
traceability rows; the live test's outcome if run; the gates and CI.
