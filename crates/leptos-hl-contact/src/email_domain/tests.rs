// tests.rs — unit tests for the parent module.
//
// Recorded DoH JSON answers (RFC 018 Amendment A3), not the network: every
// literal here is trimmed from the step-0 spike's own report, with domain
// names replaced by fictional ones — nothing here queries a real resolver.
// The two resolvers' shapes differ in two small, deliberate ways (A4): one
// appends a trailing `.` to `name` and adds a `Comment` field the other
// omits.  This module reads only `Status`, `Answer[].type` and
// `Answer[].data`, so both shapes must decide the same way.

use std::time::Duration;

use tokio::net::TcpListener;

use super::*;

fn parse(json: &str) -> Answer {
    serde_json::from_str(json).expect("recorded fixture parses")
}

fn config(resolver_url: &str) -> EmailDomainCheck {
    EmailDomainCheck::new(resolver_url)
}

// ---------------------------------------------------------------------------
// Recorded fixtures (RFC 018 Amendment A2's table)
// ---------------------------------------------------------------------------

/// (a) MX present, style 1 (no trailing dot, no `Comment`).
const MX_PRESENT: &str = r#"{"Status":0,"TC":false,"RD":true,"RA":true,"AD":false,"CD":false,
 "Question":[{"name":"mail.example","type":15}],
 "Answer":[{"name":"mail.example","type":15,"TTL":300,
            "data":"0 mx.mail.example."}]}"#;

/// (a) the same, style 2 (a trailing dot on `name`, and a `Comment`).
const MX_PRESENT_OTHER_RESOLVER: &str = r#"{"Status":0,"TC":false,"RD":true,"RA":true,"AD":false,"CD":false,
 "Question":[{"name":"mail.example.","type":15}],
 "Answer":[{"name":"mail.example.","type":15,"TTL":180,
            "data":"0 mx.mail.example."}],
 "Comment":"Response from 198.51.100.1."}"#;

/// (b) no MX record — only the `CNAME` the name resolves through — for
/// `cdn-host.example`'s MX query.
const NO_MX_CNAME_ONLY: &str = r#"{"Status":0,"...":"...","Answer":[{"name":"cdn-host.example","type":5,
  "data":"cdn-host.example.cdn.example.net."}]}"#;

/// (b) the address query for the same name: the `CNAME` plus an `A` record.
const ADDRESS_PRESENT: &str = r#"{"Status":0,"...":"...","Answer":[
  {"name":"cdn-host.example","type":5,"data":"cdn-host.example.cdn.example.net."},
  {"name":"cdn-host.example.cdn.example.net","type":1,"data":"203.0.113.9"}]}"#;

/// (c) NODATA: no `Answer` array at all, only an `Authority` SOA.
const NODATA: &str = r#"{"Status":0,"...":"...","Authority":[{"name":"no-route.example","type":6,
  "data":"ns1.no-route.example. hostmaster.no-route.example. 1 1800 900 604800 1800"}]}"#;

/// (d) a null MX (RFC 7505): preference 0, exchange the root.
const NULL_MX: &str = r#"{"Status":0,"...":"...","Answer":[{"name":"no-mail.example","type":15,"TTL":300,
  "data":"0 ."}]}"#;

/// (e) NXDOMAIN.
const NXDOMAIN: &str = r#"{"Status":3,"...":"...","Authority":[{"name":"","type":6,"data":"a.root-servers.net. x. 1 1800 900 604800 86400"}]}"#;

/// (e) SERVFAIL.
const SERVFAIL: &str = r#"{"Status":2,"...":"...","Comment":["EDE(9): DNSKEY Missing no SEP matching the DS found."]}"#;

// ---------------------------------------------------------------------------
// The decision table (Amendment A2), each row from both resolver shapes
// where they differ
// ---------------------------------------------------------------------------

#[test]
fn an_mx_record_present_accepts() {
    for json in [MX_PRESENT, MX_PRESENT_OTHER_RESOLVER] {
        assert_eq!(verdict(&parse(json), None), DomainVerdict::Accept, "{json}");
    }
}

#[test]
fn no_mx_but_an_address_record_accepts() {
    let mx = parse(NO_MX_CNAME_ONLY);
    let address = parse(ADDRESS_PRESENT);
    assert_eq!(verdict(&mx, Some(&address)), DomainVerdict::Accept);
}

#[test]
fn no_mx_and_no_address_record_rejects() {
    let mx = parse(NO_MX_CNAME_ONLY);
    let address = parse(NODATA);
    assert_eq!(verdict(&mx, Some(&address)), DomainVerdict::Reject);
}

#[test]
fn nodata_on_the_mx_query_with_no_address_answer_at_all_rejects() {
    // The address query was never made (the caller's own decision, not
    // this function's) — the pure decision still has enough to reject.
    let mx = parse(NODATA);
    assert_eq!(verdict(&mx, None), DomainVerdict::Reject);
}

#[test]
fn a_null_mx_rejects() {
    assert_eq!(verdict(&parse(NULL_MX), None), DomainVerdict::Reject);
}

#[test]
fn nxdomain_rejects() {
    assert_eq!(verdict(&parse(NXDOMAIN), None), DomainVerdict::Reject);
}

#[test]
fn servfail_is_unknown_not_a_rejection() {
    assert_eq!(
        verdict(&parse(SERVFAIL), None),
        DomainVerdict::Unknown("servfail")
    );
}

/// An RCODE this module does not specifically name (FORMERR, 1) is treated
/// the same as SERVFAIL: the resolver said something is wrong, not the
/// domain.
#[test]
fn an_unrecognised_rcode_is_unknown_not_a_rejection() {
    let formerr = r#"{"Status":1,"...":"..."}"#;
    assert_eq!(
        verdict(&parse(formerr), None),
        DomainVerdict::Unknown("servfail")
    );
}

/// NODATA on the address query too (both queries NOERROR, neither carries a
/// route) still rejects, not `Unknown` — this is the domain saying no, not
/// a lookup failure.
#[test]
fn nodata_on_both_queries_rejects() {
    let mx = parse(NO_MX_CNAME_ONLY);
    let address = parse(NODATA);
    assert_eq!(verdict(&mx, Some(&address)), DomainVerdict::Reject);
}

/// SERVFAIL on the address query (the MX query itself decided nothing) is
/// `Unknown`, not a rejection.
#[test]
fn servfail_on_the_address_query_is_unknown() {
    let mx = parse(NO_MX_CNAME_ONLY);
    let address = parse(SERVFAIL);
    assert_eq!(
        verdict(&mx, Some(&address)),
        DomainVerdict::Unknown("servfail")
    );
}

// ---------------------------------------------------------------------------
// The lookup: every failure accepts (as `Unknown`, D1's rule)
// ---------------------------------------------------------------------------

fn request_config() -> EmailDomainCheck {
    config("http://127.0.0.1:1/dns-query") // never dialled; only its builder fields are read
}

/// Answer `body` once, on a local listener, and refuse any further
/// connection: the listener is dropped as soon as the first is accepted, so
/// a second query — the address fallback, made when it should not be —
/// fails fast with a refused connection instead of hanging or, worse,
/// silently succeeding against a second accept this helper never offers.
async fn respond_once(status_line: &'static str, body: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dns-query", listener.local_addr().unwrap());
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut socket, _) = listener.accept().await.unwrap();
        drop(listener);
        let mut discard = [0u8; 1024];
        let _ = socket.read(&mut discard).await;
        let head = format!(
            "{status_line}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
        socket.shutdown().await.ok();
    });
    url
}

#[tokio::test]
async fn an_mx_answer_over_the_wire_decides_accept() {
    let url = respond_once("HTTP/1.1 200 OK", MX_PRESENT.as_bytes()).await;
    let verdict = check("mail.example", &config(&url)).await;
    assert_eq!(verdict, DomainVerdict::Accept);
}

#[tokio::test]
async fn a_null_mx_answer_over_the_wire_decides_reject() {
    let url = respond_once("HTTP/1.1 200 OK", NULL_MX.as_bytes()).await;
    let verdict = check("no-mail.example", &config(&url)).await;
    assert_eq!(verdict, DomainVerdict::Reject);
}

/// The address query is not made when the MX answer already decides (an MX
/// record present): `respond_once`'s listener refuses any connection past
/// its first, so a spurious second query would turn the verdict into
/// `Unknown("transport")` instead of `Accept`.
#[tokio::test]
async fn the_address_query_is_skipped_when_mx_already_decides() {
    let url = respond_once("HTTP/1.1 200 OK", MX_PRESENT.as_bytes()).await;
    let verdict = check("mail.example", &config(&url)).await;
    assert_eq!(
        verdict,
        DomainVerdict::Accept,
        "a second (refused) query would have produced Unknown(\"transport\") instead"
    );
}

/// A malformed body (not the DoH JSON shape at all) is `Unknown("unparsable")`,
/// never a rejection.
#[tokio::test]
async fn an_unparsable_body_is_unknown_not_a_rejection() {
    let url = respond_once("HTTP/1.1 200 OK", b"not json at all").await;
    let verdict = check("mail.example", &config(&url)).await;
    assert_eq!(verdict, DomainVerdict::Unknown("unparsable"));
}

/// Handoff 02 review, C1: a crafted "domain" that would inject extra query
/// parameters is refused **before** any request is sent.  Proof: the
/// listener would answer with an MX record — `Accept` — if it were ever
/// contacted; the verdict is `Unknown("unparsable")` instead, so the
/// request never reached it.
#[tokio::test]
async fn a_domain_that_is_not_a_hostname_is_refused_without_a_request() {
    let url = respond_once("HTTP/1.1 200 OK", MX_PRESENT.as_bytes()).await;
    let crafted = "example.com&type=TXT";
    let verdict = check(crafted, &config(&url)).await;
    assert_eq!(verdict, DomainVerdict::Unknown("unparsable"));
}

/// A non-2xx HTTP status (the resolver itself errored, not a DNS-level
/// SERVFAIL) is `Unknown("transport")`, never a rejection.
#[tokio::test]
async fn a_non_2xx_status_is_unknown_transport() {
    let url = respond_once("HTTP/1.1 500 Internal Server Error", b"{}").await;
    let verdict = check("mail.example", &config(&url)).await;
    assert_eq!(verdict, DomainVerdict::Unknown("transport"));
}

/// A resolver that never answers is abandoned at the configured timeout —
/// proven the way the challenge verifier's silent-server test proves it
/// (`challenge::http::tests::a_silent_server_is_a_timeout`): accept the
/// connection and hold it well past the caller's own limit.
#[tokio::test]
async fn a_silent_resolver_times_out_at_the_configured_limit() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dns-query", listener.local_addr().unwrap());
    let _held = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(socket);
    });

    let start = std::time::Instant::now();
    let verdict = check(
        "mail.example",
        &config(&url).with_timeout(Duration::from_millis(200)),
    )
    .await;
    let elapsed = start.elapsed();

    assert_eq!(verdict, DomainVerdict::Unknown("timeout"));
    assert!(
        elapsed < Duration::from_secs(2),
        "the cap held, got {elapsed:?}"
    );
}

#[test]
fn the_default_timeout_is_two_seconds() {
    assert_eq!(EmailDomainCheck::DEFAULT_TIMEOUT, Duration::from_secs(2));
    let c = request_config();
    let overridden = c.with_timeout(Duration::from_millis(500));
    assert_eq!(overridden.timeout, Duration::from_millis(500));
}

/// Handoff 02 review, C1: the LDH check itself, directly.
#[test]
fn looks_like_a_hostname_accepts_ldh_and_refuses_everything_else() {
    for hostname in ["example.com", "mail.example.co.uk", "a-b.example", "x"] {
        assert!(looks_like_a_hostname(hostname), "{hostname}");
    }
    for not_a_hostname in [
        "",
        "example.com&type=TXT",
        "example.com?x=1",
        "example.com/../x",
        "exam ple.com",
        "example.com\r\nHost: evil.example",
        "例え.com",
    ] {
        assert!(!looks_like_a_hostname(not_a_hostname), "{not_a_hostname}");
    }
}

// ---------------------------------------------------------------------------
// Live: a real resolver (RFC 018 D6)
// ---------------------------------------------------------------------------

/// Two real lookups against a real DNS-over-HTTPS resolver, for the two
/// shapes whose answers are stable enough to assert on: a well-known domain
/// with an MX record, and RFC 7505's own null-MX example.
///
/// Skipped by default.  Run it explicitly:
///
/// ```bash
/// cargo test -p leptos-hl-contact --all-features --lib -- --ignored email_domain::tests::live_
/// ```
///
/// No key or secret is needed — a public DoH endpoint takes none.  Asserts
/// nothing about either domain beyond the one shape each is known for; a
/// third-party domain's other records can change at any time.
#[tokio::test]
#[ignore = "queries a real resolver"]
async fn live_a_well_known_domain_with_an_mx_record_accepts() {
    let config = EmailDomainCheck::new("https://cloudflare-dns.com/dns-query");
    let verdict = check("github.com", &config).await;
    assert_eq!(verdict, DomainVerdict::Accept, "{verdict:?}");
}

/// RFC 7505's own null-MX example, queried live.
#[tokio::test]
#[ignore = "queries a real resolver"]
async fn live_a_null_mx_domain_rejects() {
    let config = EmailDomainCheck::new("https://cloudflare-dns.com/dns-query");
    let verdict = check("example.com", &config).await;
    assert_eq!(verdict, DomainVerdict::Reject, "{verdict:?}");
}

// ---------------------------------------------------------------------------
// Break checks (required by the handoff), reported in the review request —
// not asserted here: each is a temporary local edit, run, and reverted.
// ---------------------------------------------------------------------------
