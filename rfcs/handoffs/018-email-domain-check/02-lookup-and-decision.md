# Handoff 018-02 — The lookup, and the decision table

**RFC.** [RFC 018](../../done/018-email-domain-check.md) D1, D4, and the Amendment (A1–A4)
**Roadmap.** P-35
**Depends on.** 01 (spike, reviewed)

## Goal

**The check, as a pure function of a resolver's answer, plus the transport
it needs.**  Nothing is wired into the submission pipeline yet — that is 03.

## Change scope

### 1. `src/http/` — a GET on both targets (A1)

- **Native:** `HttpClient::get(url, headers, limit)`, mirroring `post`: no
  redirects, the caller's timeout, the same capped chunked read, no body.
- **wasm32:** `fetch::send` takes the verb, or gains a sibling for GET.
  `RequestRedirect::Manual`, the abort guard and the cap stay exactly as
  they are.
- **No behaviour change for the existing callers.**  `challenge-http` and
  `delivery-resend` must be untouched: their suites are the gate.
- **Feature gating:** `mod http` currently turns on with `challenge-http`;
  it must also turn on with the new feature.  Check every combination in the
  gates below.

### 2. `src/email_domain.rs` (new), behind `email-domain-check`

**The feature:** `email-domain-check = ["ssr", …]`, naming the same optional
dependencies the other HTTP features name.

**The config, per D4 — a constructor and builders, `#[non_exhaustive]`:**

```rust,ignore
#[non_exhaustive]
pub struct EmailDomainCheck { /* private */ }

impl EmailDomainCheck {
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
    /// The DoH endpoint the site trusts.  The crate ships no default.
    pub fn new(resolver_url: impl Into<String>) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;
}
```

- **Rustdoc must say:** the domain leaves the server, the local part never
  does; the site chooses the resolver and should say so in its privacy
  notice; a failure of the lookup accepts the submission.
- **`Debug`:** the URL and the timeout, nothing else to redact.

**The decision, as its own function:**

```rust,ignore
pub(crate) enum DomainVerdict { Accept, Reject, Unknown(&'static str) }

/// Decides from one MX answer, and (only when it must) one address answer.
pub(crate) fn verdict(mx: &Answer, address: Option<&Answer>) -> DomainVerdict;
```

- **The table is A2's,** including NODATA and null MX.
- **`Unknown`** carries a fixed reason for the log: `"servfail"`,
  `"timeout"`, `"transport"`, `"unparsable"`.  The caller turns it into an
  accept.
- **Parse tolerantly** (A4): unknown fields ignored, no assumption about a
  trailing `.`.

**The lookup itself:** one GET per query, the MX query always, the address
query **only** when the MX answer has no type-15 record.

### 3. Tests (unit)

- **Recorded answers, not the network** (A3).  Put the JSON from the spike
  report in the test module as literals, trimmed, with no real person's
  domain.
- **Every row of A2's table,** each from both resolvers' shapes where they
  differ (the trailing dot, the extra field).
- **The address query is not made** when the MX answer already decides.
- **Every failure accepts:** SERVFAIL, a timeout, a transport error, an
  unparsable body.
- **The timeout** is the configured one, proven as the challenge's
  silent-server test proves it.
- **Break checks, required:**
  - make SERVFAIL reject → the accept test fails;
  - drop the null-MX case → its test fails.

### 4. `CHANGELOG.md` `[Unreleased]`

**Added:** the feature exists, is off unless configured, and what it decides.
Handoff 03 replaces this line with the shipped behaviour.

## Gates

- The shared gates on both toolchains; the MSRV checks; **feature
  combinations**: the new feature alone, with `challenge-http`, with
  `delivery-resend`, and a build with none of them; both wasm suites with
  counts; the examples with `--locked`; CI.

## Review request

`.git-exclude/review-request/018-email-domain-check/02-lookup-and-decision.md`:
the commit; the GET addition on both targets and proof the existing callers
are unchanged; the decision table as implemented; the tests with results;
both break checks; the feature-combination results; the gates and CI.
