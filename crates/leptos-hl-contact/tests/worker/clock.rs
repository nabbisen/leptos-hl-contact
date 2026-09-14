//! The form token's clock on a wasm32 server (RFC 011 D4).

use leptos_hl_contact::{FormTokenConfig, FormTokenError, issue_form_token, verify_form_token};
use wasm_bindgen_test::wasm_bindgen_test;

fn config(min_age_secs: u64) -> FormTokenConfig {
    FormTokenConfig::new(b"worker-test-secret-0123456789abcdef".to_vec()).with_min_age(min_age_secs)
}

/// FR-VAL-06, FR-ABUSE-03, NFR-PORT-02 (RFC 011 D4): issuing reads the
/// JavaScript clock without panicking, and a token issued now verifies now.
///
/// Issuing and verifying read the same clock, so a clock stuck at 0 would
/// still verify; the token's timestamp (`{timestamp}|{nonce}|{hmac}`) is
/// therefore also checked to be within 2 s of the JavaScript clock.
#[wasm_bindgen_test]
fn a_token_issued_now_verifies() {
    let config = config(0);
    let token = issue_form_token(&config);
    let result = verify_form_token(&token.0, None, &config);
    assert!(result.is_ok(), "{result:?}");

    let signed_at: u64 = token.0.split('|').next().unwrap().parse().unwrap();
    let js_now = (js_sys::Date::now() / 1000.0) as u64;
    assert!(
        signed_at.abs_diff(js_now) <= 2,
        "signed at {signed_at}, JavaScript clock {js_now}"
    );
}

/// FR-VAL-06, FR-ABUSE-13, NFR-PORT-02 (RFC 011 D4): the minimum age is
/// measured on the same clock.
#[wasm_bindgen_test]
fn a_token_is_too_young_before_its_minimum_age() {
    let config = config(3600);
    let token = issue_form_token(&config);
    let result = verify_form_token(&token.0, None, &config);
    assert!(
        matches!(result, Err(FormTokenError::TooYoung)),
        "{result:?}"
    );
}
