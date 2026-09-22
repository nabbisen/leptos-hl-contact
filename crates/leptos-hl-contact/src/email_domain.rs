// email_domain.rs — an opt-in check that the visitor's email domain can
// receive mail at all, over DNS over HTTPS (RFC 018), through the shared
// HTTP transport (`src/http.rs`).
//
// What this decides, and does not (RFC 018 Summary):
// - Can: catch a domain with no mail route — typos and dead domains.
// - Cannot: prove a mailbox exists.  Only sending can, and this module
//   never sends probe mail.
// - Must never: refuse a submission because *our* lookup failed.  Every
//   failure of the lookup — a timeout, a transport error, an unparsable
//   answer, or the resolver's own SERVFAIL — accepts (D1).
//
use std::time::Duration;

use serde::Deserialize;

use crate::http::{HttpClient, HttpError};

/// DNS RCODEs this module reads from a DoH JSON `Status` field.  Any other
/// value is treated the same as `SERVFAIL`: the resolver, not the domain,
/// said something is wrong.
const RCODE_NOERROR: u16 = 0;
const RCODE_NXDOMAIN: u16 = 3;

/// DNS record types this module reads from `Answer[].type`.
const RECORD_TYPE_A: u16 = 1;
const RECORD_TYPE_MX: u16 = 15;
const RECORD_TYPE_AAAA: u16 = 28;

// ---------------------------------------------------------------------------
// Configuration (RFC 018 D4)
// ---------------------------------------------------------------------------

/// Configuration for the opt-in check that an email address's domain can
/// receive mail at all (RFC 018).
///
/// **The domain leaves the server** on every checked submission; the local
/// part of the address never does.  The resolver is a third party that
/// sees which domains your visitors use — name it in your privacy notice.
///
/// **The crate ships no default resolver.**  Which third party sees your
/// visitors' email domains is your decision, not this crate's.
///
/// **A failure of the lookup itself accepts the submission.**  A timeout,
/// a transport error, an unparsable answer, or the resolver's own
/// `SERVFAIL` never refuses a visitor — only the domain itself, by
/// answering `NXDOMAIN`, offering no route at all, or stating a null MX
/// (RFC 7505), does that.
///
/// Provided in context, like every other optional server piece; absent, the
/// check does not run.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct EmailDomainCheck {
    resolver_url: String,
    timeout: Duration,
}

impl EmailDomainCheck {
    /// The default lookup timeout: 2 seconds.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

    /// A check against `resolver_url`, the DNS-over-HTTPS (JSON media type)
    /// endpoint the site trusts, for example `"https://cloudflare-dns.com/dns-query"`
    /// or `"https://dns.google/resolve"`.
    pub fn new(resolver_url: impl Into<String>) -> Self {
        Self {
            resolver_url: resolver_url.into(),
            timeout: Self::DEFAULT_TIMEOUT,
        }
    }

    /// Overrides the lookup timeout.  Default: [`Self::DEFAULT_TIMEOUT`].
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

// ---------------------------------------------------------------------------
// The DoH JSON answer, parsed tolerantly (Amendment A4)
// ---------------------------------------------------------------------------

/// One resolver's JSON answer to one query.
///
/// Parsed tolerantly: unknown fields (a resolver-specific `Comment`, for
/// example) are ignored, and no assumption is made about whether a `name`
/// carries a trailing `.` — this module never reads `name` at all.
#[derive(Deserialize)]
pub(crate) struct Answer {
    #[serde(rename = "Status")]
    status: u16,
    /// Absent entirely on a NODATA answer (an `Authority` SOA only); `default`
    /// makes that an empty list rather than a parse failure.
    #[serde(rename = "Answer", default)]
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}

// ---------------------------------------------------------------------------
// The decision (RFC 018 D1, Amendment A2)
// ---------------------------------------------------------------------------

/// What the domain check decides.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DomainVerdict {
    /// The domain has a mail route: an MX record, or (with no MX) an
    /// address record (RFC 5321 §5.1's implicit MX).
    Accept,
    /// The domain itself says mail cannot arrive: NXDOMAIN, NODATA, or a
    /// null MX (RFC 7505).
    Reject,
    /// The lookup did not produce a trustworthy answer.  The caller must
    /// accept the submission and may log this reason — never the address,
    /// never the domain.
    Unknown(&'static str),
}

/// Decides from one MX answer, and — only when the MX answer carries no MX
/// record at all — one address (`A`) answer.
///
/// A pure function of two already-parsed answers: it makes no request and
/// applies no timeout itself.
pub(crate) fn verdict(mx: &Answer, address: Option<&Answer>) -> DomainVerdict {
    match mx.status {
        RCODE_NXDOMAIN => return DomainVerdict::Reject,
        RCODE_NOERROR => {}
        // SERVFAIL, and every other RCODE this module does not specifically
        // recognise: the resolver said something is wrong, not the domain.
        _ => return DomainVerdict::Unknown("servfail"), // SERVFAIL (2) included
    }

    let has_mx = mx.records.iter().any(|r| r.kind == RECORD_TYPE_MX);
    if !has_mx {
        // NODATA on the MX query (possibly a bare CNAME, RFC 5321 §5.1):
        // an address record still routes mail there.
        return match address {
            Some(address) => match address.status {
                RCODE_NOERROR
                    if address
                        .records
                        .iter()
                        .any(|r| r.kind == RECORD_TYPE_A || r.kind == RECORD_TYPE_AAAA) =>
                {
                    DomainVerdict::Accept
                }
                RCODE_NOERROR | RCODE_NXDOMAIN => DomainVerdict::Reject,
                _ => DomainVerdict::Unknown("servfail"),
            },
            // No address answer to fall back on: nothing says mail routes
            // here.
            None => DomainVerdict::Reject,
        };
    }

    // A null MX (RFC 7505): every MX record is preference 0, exchange ".".
    if mx
        .records
        .iter()
        .all(|r| r.kind != RECORD_TYPE_MX || r.data == "0 .")
    {
        return DomainVerdict::Reject;
    }
    DomainVerdict::Accept
}

// ---------------------------------------------------------------------------
// The lookup
// ---------------------------------------------------------------------------

/// Looks up `domain`'s mail route and decides its verdict, within
/// `config`'s timeout.
///
/// One GET for the MX query, always; a second, for the address record,
/// only when the MX answer carries no MX record at all — never more than
/// two requests, and never a query per keystroke (that is the caller's
/// responsibility: this function is called once per submission, D2).
pub(crate) async fn check(domain: &str, config: &EmailDomainCheck) -> DomainVerdict {
    let mx = match query(domain, "MX", config).await {
        Ok(answer) => answer,
        Err(reason) => return DomainVerdict::Unknown(reason),
    };

    // The address fallback matters only on NODATA (`verdict`'s own branch);
    // anything else — an MX record present, NXDOMAIN, SERVFAIL — already
    // has enough to decide.
    let needs_address_fallback =
        mx.status == RCODE_NOERROR && !mx.records.iter().any(|r| r.kind == RECORD_TYPE_MX);
    if !needs_address_fallback {
        return verdict(&mx, None);
    }

    match query(domain, "A", config).await {
        Ok(address) => verdict(&mx, Some(&address)),
        Err(reason) => DomainVerdict::Unknown(reason),
    }
}

/// One DoH JSON GET, over the shared transport.
///
/// The domain is already syntax-validated (FR-VAL-02) by every caller in
/// this crate before this runs: it holds no `&`, `=`, `?` or whitespace, so
/// it needs no percent-encoding to sit safely in a query string.
async fn query(
    domain: &str,
    record_type: &str,
    config: &EmailDomainCheck,
) -> Result<Answer, &'static str> {
    let url = format!("{}?name={domain}&type={record_type}", config.resolver_url);
    let client = HttpClient::new();
    let response = client
        .get(
            &url,
            &[("accept", "application/dns-json".to_string())],
            config.timeout,
        )
        .await
        .map_err(|error| match error {
            HttpError::Timeout => "timeout",
            HttpError::Transport(_) | HttpError::Unusable(_) => "transport",
        })?;
    if response.status != 200 {
        return Err("transport");
    }
    serde_json::from_slice::<Answer>(&response.body).map_err(|_| "unparsable")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
