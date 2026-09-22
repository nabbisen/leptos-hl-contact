# Email Domain Check

An opt-in check that the email address's domain can receive mail at all,
so a visitor who mistypes `gmial.com`, or types a domain that stopped
existing, is told so while still on the page — instead of the site owner
answering into the void.

**What it can and cannot do:**
- **Can:** catch a domain with no mail route — typos and dead domains.
- **Cannot:** prove a mailbox exists.  Only sending can, and the crate
  never sends probe mail.
- **Must never:** refuse a submission because *our* lookup failed.  A
  timeout, a transport error, an unparsable answer, or the resolver's own
  `SERVFAIL` always accepts — only the domain itself, by saying it has no
  route, refuses a visitor.

## The API

```rust,ignore
pub struct EmailDomainCheck { /* private */ }

impl EmailDomainCheck {
    pub const DEFAULT_TIMEOUT: Duration; // 2 seconds
    /// The DNS-over-HTTPS (JSON media type) endpoint the site trusts.
    /// The crate ships no default.
    pub fn new(resolver_url: impl Into<String>) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;
}
```

Provide it in the context closure passed to `leptos_routes_with_context`,
like every other optional server piece:

```rust,ignore
use leptos_hl_contact::email_domain::EmailDomainCheck;

provide_context(EmailDomainCheck::new("https://cloudflare-dns.com/dns-query"));
```

Requires the `email-domain-check` feature.  Absent from context, the check
does not run — every site keeps today's behaviour unless it opts in.

## What it decides

Runs once per submission, after field validation and before the server
policy and the challenge — so a bot cannot make the server do lookups, and
a visitor who mistyped is never asked to solve a challenge first.  The
lookup is skipped entirely when the address already failed syntax
validation.

| The domain | Outcome |
|------------|---------|
| Has an MX record, not a null MX | accept |
| No MX, but an A or AAAA record | accept (RFC 5321 §5.1's implicit MX) |
| A null MX (RFC 7505: the domain states it takes no mail) | **reject** |
| NXDOMAIN, or no record at all | **reject** |
| The resolver answers `SERVFAIL`, times out, or answers something this crate cannot parse | **accept**, with a `warn` log naming the reason |

A rejection is reported under the email field as `email_domain`
(`FieldErrorCode::EmailDomain`), distinct from `format_email` — it tells
the visitor what is actually wrong: not that the address is malformed, but
that its domain has nowhere for mail to go.  See
[Localization](../guides/localization.md) for the label, and
[Customization](../guides/customization.md#contacterrorlabels) for the
full label list.

## Privacy

The lookup sends data about the visitor's submission to a third party.
Say so in your privacy notice, and name the resolver.

- **The domain leaves the server** on every checked submission — for
  example `example.com` out of `visitor@example.com` — sent to whichever
  DNS-over-HTTPS resolver you configured.  **The local part never does:**
  the resolver never sees `visitor@`, only `example.com`.
- **The site chooses the resolver.**  The crate ships no default: which
  third party sees your visitors' email domains is your decision.  Public
  resolvers publish their own privacy policies — for example
  [Cloudflare's](https://www.cloudflare.com/privacypolicy/) or
  [Google's](https://policies.google.com/privacy) — read the one for the
  resolver you name.
- **Nothing is cached.**  Every submission is a fresh lookup; the crate
  keeps no record of which domains it has seen.
- **Logs never carry the address or the domain,** whichever outcome the
  lookup reaches — only the decision, and for a failure, the fixed reason
  (`servfail`, `timeout`, `transport`, `unparsable`).
