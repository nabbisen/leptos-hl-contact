# Handoff 017-03 — Workers, documentation, records, the live test

**RFC.** [RFC 017](../../accepted/017-http-api-delivery.md) D5, D6 (the live test), D7
**Roadmap.** P-22
**Depends on.** 02 (approved)

## Change scope

### 1. The live test (RFC D6)

- **Where:** beside the `#[ignore]`d live vendor challenge tests, in the
  same style.
- **What it does:** with `RESEND_API_KEY` and `RESEND_FROM` set, send one
  real request to **`delivered@resend.dev`**, Resend's documented test
  recipient, which simulates a delivered message (Resend, "Send test
  emails", checked 2026-09-22).  `RESEND_TO` overrides the recipient.
- **It asserts:** success, and that a message id was returned.
- **Rustdoc:** how to run it, that it is skipped by default, and that it
  sends no mail to a real person unless `RESEND_TO` says so.
- **Never** put a key in the test, a fixture or the request.

### 2. Documentation (RFC D7)

**The one thing an integrator must not have to guess is which backend to
use.**

| Page | Change |
|------|--------|
| `guides/delivery-backends.md` | opens with a table: **SMTP** (native only; a relay you run or rent), **Resend** (native and Workers; an API key), **your own** (any target).  Then a section for the adapter: the config, the three required values, what happens when one is missing, the deadline and how it composes with `DeliveryTimeout`, that the body is the same as SMTP's, and that errors carry a status code only |
| `guides/cloudflare-workers.md` | "Delivery on a Worker" stops saying bring your own backend: the adapter is the built-in option there, with the SMTP row still explaining why SMTP cannot run.  Keep the custom-backend paragraph for sites that need one |
| `reference/feature-flags.md` | a `delivery-resend` row, with its Workers column |
| `reference/api.md` | `ResendDelivery`, `ResendConfig`, its constructor and builders, the default timeout, and `with_url`.  **Use the full path** `delivery::resend::{…}`: unlike most types, the SMTP and Resend backends are not re-exported at the crate root |
| `getting-started/production-checklist.md` | one row: the API key comes from the environment, the sender's domain is verified at the provider, and a delivery failure has been seen once in staging |
| `security/hardening.md` or `security/README.md` | one line: an API key is a secret like the SMTP password, and the crate redacts it in `Debug`.  Add that an overridden endpoint (`with_url`, `with_verify_url`) carries that secret, so it must be `https` outside local testing — the crate warns when it is not (handoff 02 C1) |
| `README.md` | the delivery bullet mentions the two built-in backends |
| `development/external-design.md` | the delivery table gains the adapter and its error mapping |
| `development/architecture.md` | the source layout and the module map, exactly |
| `development/testing.md` | the traceability rows for the new tests, and the live test named as `#[ignore]`d |

**Say plainly what the crate does not do:** no retries, no queue, no
attachments, and one provider only.

### 3. Records

- **`requirements.md` is mine,** not yours: list the rows you think need a
  status change, and I will write them at review.
- **`CHANGELOG.md`:** a *Documentation* entry, and check the whole RFC 017
  entry set reads as one feature.

## Gates

- **The shared gates,** the MSRV checks, both wasm suites, the examples with
  `--locked`, `mdbook build docs` with no warnings, and CI.
- **The live test:** say whether you ran it.  If you did, report the status
  and that a message id came back — **never the key**.

## Review request

`.git-exclude/review-request/017-http-api-delivery/03-workers-docs-and-records.md`:
- the commit;
- each page's change;
- the decision table as written;
- the live test's outcome, if run;
- the requirements rows you propose;
- the gates and CI.
