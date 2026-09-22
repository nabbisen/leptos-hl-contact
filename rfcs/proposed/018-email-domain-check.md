# RFC 018 — An opt-in check that the email domain can receive mail

**Status.** Proposed — 2026-09-22.  Milestone M8, authorised by the owner
2026-09-22 (P-35 as the theme).
**Tracks.** Roadmap P-35.  Requirements FR-VAL-02, FR-VAL-07, FR-OBS-01/02,
NFR-PORT-02, NFR-PRIV-*.
**Touches.** a new module behind a new feature, `model.rs`/`server.rs` (one
step in the pipeline), `error.rs` and `config.rs` (how the visitor is told),
`src/http/` (the lookup's transport), tests at every layer, docs,
`CHANGELOG.md`.

## Summary

A visitor types `gmial.com`, or a domain that stopped existing.  The form
accepts it, the site owner answers into the void, and nobody learns
anything.  This RFC adds an **opt-in** check that the address's domain can
receive mail at all, reported under the email field while the visitor is
still on the page.

**What it can and cannot do:**
- **Can:** catch a domain with no mail route — typos and dead domains.
- **Cannot:** prove a mailbox exists.  Only sending can, and the crate must
  not send probe mail.
- **Must never:** refuse a submission because *our* lookup failed.

## Why now

- **It is the owner's original question** (2026-09-13), recorded as P-35 and
  approved at low priority.
- **The transport now exists.**  RFC 017 left a private HTTPS client that
  works natively and on a Worker.  A Worker cannot do ordinary DNS — it has
  no UDP — so before that module, this feature could not have worked on our
  newest supported target.

## Design

### Step 0 — Spike, before any other work

**The question:** can one lookup path serve both targets?

- **The candidate:** DNS over HTTPS (DoH) with the JSON media type, through
  `src/http/`'s existing POST/GET path, against a resolver the site chooses.
- **The spike must show:**
  1. an MX lookup for a domain that has one, on both targets;
  2. a domain with **no** MX and **an** address record (RFC 5321 §5.1 says
     mail still goes there);
  3. a domain with neither;
  4. **a null MX** (`.`, RFC 7505): the domain states it accepts no mail;
  5. NXDOMAIN versus SERVFAIL, distinguished — the first is the visitor's
     mistake, the second is ours;
  6. a resolver that does not answer: the caller's time limit applies and
     the request is abandoned;
  7. what the answer costs: bytes and round trips.
- **Stop conditions:** the JSON shape cannot be parsed portably; the shared
  transport needs a GET it does not have and adding one is more than a small
  change; or a Worker cannot reach the resolver.
- **Report only.**  No production code.

### D1 — What the check decides

| Case | Outcome |
|------|---------|
| MX present, not a null MX | accept |
| No MX, but an A or AAAA record | accept (RFC 5321 §5.1 implicit MX) |
| Null MX (`.`) | **reject:** the domain says it takes no mail |
| NXDOMAIN, or no record at all | **reject** |
| SERVFAIL, timeout, transport error, unparsable answer | **accept**, with a `warn` log naming the reason and **not** the address |

**The rule behind the table:** the visitor is refused only when the domain
itself says mail cannot arrive.  Every failure of ours is the visitor's
benefit of the doubt.

### D2 — Where it runs

- **After syntax validation** (FR-VAL-02 already rejects address literals and
  single-label domains), and **after the honeypot and the form token**, so a
  bot cannot make us do lookups.
- **Before the challenge,** so a visitor who mistyped is not asked to solve a
  challenge first.
- **Once per submission,** never per keystroke.
- **The lookup is skipped entirely** when the address failed syntax
  validation.

### D3 — How the visitor is told

Two shapes, and this is **owner question 2**:
- **(a) Reuse `format_email`.**  The visitor sees "Enter a valid email
  address."  No new label, no new code, nothing to translate.
- **(b) A new code and label**, for example `email_domain`: "We could not
  find a mail server for that domain.  Please check the spelling."
  - **Better UX:** it tells the visitor *what* is wrong, which is the point
    of the feature.
  - **The cost:** a new variant in a public enum and a new field in
    `ContactErrorLabels` — a migration line for anyone who writes either as
    a literal, and a string every translated site must add.

**Recommendation: (b).**  "Enter a valid email address" in front of an
address that *is* syntactically valid is exactly the confusing message this
project tries not to ship.

### D4 — Configuration

```rust,ignore
#[non_exhaustive]
pub struct EmailDomainCheck { /* private */ }

impl EmailDomainCheck {
    /// The resolver's DoH endpoint; no default is assumed for the site.
    pub fn new(resolver_url: impl Into<String>) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;   // DEFAULT_TIMEOUT = 2 s
    pub const DEFAULT_TIMEOUT: Duration;
}
```

- **Provided in context,** like every other optional server piece; absent
  means the check does not run.
- **The site chooses the resolver.**  We ship no default endpoint: which
  third party sees your visitors' email domains is the site's decision, not
  ours (owner question 3).
- **The feature** is `email-domain-check`, implying `ssr` and the shared
  transport's dependencies.
- **The builder shape** follows `ResendConfig`, so a later field needs no
  migration.

### D5 — Privacy, and what is logged

- **What leaves the server:** the **domain**, never the local part, never the
  whole address.
- **The site must disclose it:** a resolver is a third party that sees which
  domains its visitors use.  The documentation says so in the same voice the
  challenge page uses for vendor privacy.
- **Logs:** the outcome and the reason, never the address and never the
  domain — a domain can identify a small employer.
- **No cache in this RFC.**  A cache keyed by domain is a store of personal
  data by another name; if it is ever wanted, it is its own RFC with its own
  privacy paragraph.

### D6 — Tests

- **Unit:** the whole D1 table against a stubbed resolver answer, including
  null MX and the SERVFAIL-accepts rule.
- **L2 (server suite):** a rejecting domain gives the field error in both
  request forms; a SERVFAIL accepts and delivers; the honeypot still wins
  first; the challenge is not reached when the address is refused.
- **Worker tests:** the lookup over the stubbed `fetch`, and a dropped
  submission abandoning it.
- **Logs:** no address, no domain, in any line.
- **Break checks:** make SERVFAIL reject → the accept test fails; skip the
  null-MX case → its test fails.
- **A live test,** `#[ignore]`d, against a real resolver for a domain we
  control the expectations of (`example.com` has no MX; a well-known domain
  has one).

## Compatibility

- **Off unless the site provides the context.**  No behaviour changes for
  anyone else.
- **Breaking, if D3 (b) is chosen:** a new `ContactErrorLabels` field and a
  new error code; both need a migration line, and the release is a minor.

## Handoffs (planned)

| # | Scope |
|---|-------|
| 01 | Step 0 spike: DoH over the shared transport, the six questions above.  Report only |
| 02 | The check itself: the module, the config, the D1 table, unit tests |
| 03 | The pipeline step, the visitor-facing error, L2 and worker tests |
| 04 | Documentation (including the privacy paragraph), traceability, CHANGELOG |

## Owner questions (recommendations first)

1. **Scope: MX with the address-record fallback and null MX, nothing more.**
   No SMTP probing, no mailbox verification, no disposable-domain lists.
2. **A distinct error code and label (D3 b),** so the visitor is told what is
   actually wrong.  The cost is one migration line and one string per
   translated site.
3. **No default resolver.**  The site names the endpoint it trusts.  The
   alternative — shipping a default — makes a privacy decision on the site's
   behalf and points its traffic at a third party it never chose.
4. **No cache** in this RFC.
5. **A 2-second default timeout**, inside the request the visitor is waiting
   on.
