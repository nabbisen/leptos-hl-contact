//! The form token in the browser: acquisition, refresh, and survival of a
//! failed submission (RFC 004 handoff 03, RFC 002).

use leptos_hl_contact::ContactFormOptions;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{Clock, FetchStub, Mounted, settle};

/// What the stubbed token endpoint answers.
const FETCHED: &str = "1700003600|00112233445566778899aabbccddeeff|ab";
/// When the mounted token was issued, in Unix seconds.
const ISSUED: u64 = 1_700_000_000;
const REFRESH_SECS: u64 = 3600;
const REFRESH_MS: i32 = 3_600_000;

fn options(token_refresh_secs: Option<u64>) -> ContactFormOptions {
    ContactFormOptions {
        token_refresh_secs,
        ..ContactFormOptions::default()
    }
}

/// A token as a server render leaves it in the field, issued at `issued`.
fn rendered_token(issued: u64) -> String {
    format!("{issued}|ffeeddccbbaa99887766554433221100|cd")
}

/// FR-UI-12, T20: with `token_refresh_secs = None` the browser never calls
/// the token endpoint, even with an empty field, and schedules nothing.
#[wasm_bindgen_test]
async fn without_refresh_the_token_endpoint_is_never_called() {
    let fetch = FetchStub::issuing(FETCHED);
    let clock = Clock::install(ISSUED);
    let form = Mounted::new(options(None));

    settle().await;

    assert_eq!(fetch.count("/api/form_token"), 0, "{:?}", fetch.urls());
    assert_eq!(clock.pending(), Vec::<i32>::new());
    assert_eq!(form.token(), "");
}

/// FR-UI-12: with refresh on, an empty field (client-side navigation)
/// acquires a token exactly once and puts it in the field.
#[wasm_bindgen_test]
async fn an_empty_token_field_acquires_exactly_once() {
    let fetch = FetchStub::issuing(FETCHED);
    let _clock = Clock::install(ISSUED);
    let form = Mounted::new(options(Some(REFRESH_SECS)));

    settle().await;
    settle().await;

    assert_eq!(fetch.count("/api/form_token"), 1, "{:?}", fetch.urls());
    assert_eq!(form.token(), FETCHED);
}

/// FR-UI-12: a rendered token already past its refresh point is refreshed
/// once — on a zero-delay timer, not synchronously — and not again: the next
/// refresh is timed from the new token's arrival.
#[wasm_bindgen_test]
async fn an_overdue_mounted_token_refreshes_once() {
    let fetch = FetchStub::issuing(FETCHED);
    let clock = Clock::install(ISSUED + REFRESH_SECS + 600);
    let form = Mounted::new(options(Some(REFRESH_SECS)));
    // As a server render leaves it, before the mount effect reads the field.
    form.set_value("form_token", &rendered_token(ISSUED));

    settle().await;
    assert_eq!(fetch.count("/api/form_token"), 0, "not synchronous");
    assert_eq!(clock.pending(), vec![0], "one immediate refresh scheduled");

    assert!(clock.fire(0));
    settle().await;
    assert_eq!(fetch.count("/api/form_token"), 1);
    assert_eq!(form.token(), FETCHED);
    assert_eq!(
        clock.pending(),
        vec![REFRESH_MS],
        "the next refresh is a full interval away"
    );
}

/// FR-UI-12, FR-ABUSE-03 (RFC 004 handoff 03 amendment): a fetched token's
/// refresh is scheduled from its arrival, on the browser's timer alone.  The
/// browser clock is moved far from the token's timestamp and changes
/// nothing; advancing a full interval fetches again, and schedules again.
#[wasm_bindgen_test]
async fn a_fetched_token_schedules_its_refresh_from_arrival() {
    let fetch = FetchStub::issuing(FETCHED);
    let clock = Clock::install(ISSUED);
    let form = Mounted::new(options(Some(REFRESH_SECS)));

    settle().await;
    assert_eq!(fetch.count("/api/form_token"), 1);
    assert_eq!(form.token(), FETCHED);
    assert_eq!(clock.pending(), vec![REFRESH_MS], "scheduled on arrival");

    // A skewed browser clock does not shorten the interval.
    clock.set_now(ISSUED + 10 * 24 * 3600);
    settle().await;
    assert_eq!(fetch.count("/api/form_token"), 1);

    clock.set_now(ISSUED + 10 * 24 * 3600 + REFRESH_SECS);
    assert!(clock.fire(REFRESH_MS), "the arrival timer is pending");
    settle().await;
    assert_eq!(fetch.count("/api/form_token"), 2);
    assert_eq!(clock.pending(), vec![REFRESH_MS], "and scheduled again");
}

/// FR-UI-12, FR-UI-07 (RFC 002): a failed submission keeps the hidden token
/// and what the visitor typed, so the next attempt can succeed.
#[wasm_bindgen_test]
async fn the_hidden_token_survives_a_failed_submission() {
    let fetch = FetchStub::install(|url| {
        if url.ends_with("/api/form_token") {
            (200, format!("\"{FETCHED}\""))
        } else {
            (500, "ServerError|contact_error:delivery_failed".to_owned())
        }
    });
    let _clock = Clock::install(ISSUED);
    let form = Mounted::new(options(Some(REFRESH_SECS)));
    settle().await;
    assert_eq!(form.token(), FETCHED);

    form.fill_valid();
    form.submit();
    settle().await;

    assert_eq!(fetch.count("/api/submit_contact"), 1, "{:?}", fetch.urls());
    assert_eq!(form.token(), FETCHED);
    assert_eq!(form.value("name"), "Ada Lovelace");
    assert_eq!(form.value("message"), "A message typed by a person.");
    assert_eq!(fetch.count("/api/form_token"), 1, "no new token needed");
}
