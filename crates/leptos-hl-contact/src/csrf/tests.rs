// tests.rs — unit tests for the CSRF token module.

use super::*;

fn test_config() -> CsrfConfig {
    CsrfConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec())
}

#[test]
fn generated_token_verifies_successfully() {
    let config = test_config();
    let token = generate_csrf_token(&config);
    assert!(verify_csrf_token(&token.0, &config));
}

#[test]
fn token_format_has_three_pipe_separated_parts() {
    let config = test_config();
    let token = generate_csrf_token(&config);
    let parts: Vec<&str> = token.0.split('|').collect();
    assert_eq!(parts.len(), 3, "token must have 3 pipe-separated parts");
}

#[test]
fn tampered_signature_fails_verification() {
    let config = test_config();
    let token = generate_csrf_token(&config);
    // Flip the last character of the signature
    let mut tampered = token.0.clone();
    let last = tampered.pop().unwrap();
    let replacement = if last == 'a' { 'b' } else { 'a' };
    tampered.push(replacement);
    assert!(!verify_csrf_token(&tampered, &config));
}

#[test]
fn wrong_secret_fails_verification() {
    let config = test_config();
    let other_config = CsrfConfig::new(b"completely-different-secret-key-xyz".to_vec());
    let token = generate_csrf_token(&config);
    assert!(!verify_csrf_token(&token.0, &other_config));
}

#[test]
fn malformed_token_fails_verification() {
    let config = test_config();
    assert!(!verify_csrf_token("", &config));
    assert!(!verify_csrf_token("not-a-token", &config));
    assert!(!verify_csrf_token("a|b", &config)); // only 2 parts
}

#[test]
fn expired_token_fails_verification() {
    let config = CsrfConfig {
        secret_key: b"test-secret".to_vec(),
        token_ttl_secs: 0, // expire immediately
    };
    let token = generate_csrf_token(&config);
    // ttl = 0 means any token older than 0 seconds is expired;
    // even a freshly generated token may be 1s old on a slow machine,
    // but this test is best-effort for the expiry path.
    // Instead, fabricate an old token directly.
    let old_ts = 1_000_000u64; // year 1970+~11 days, definitely expired
    let payload = format!("{old_ts}|aabbccddeeff00112233445566778899");
    let sig = super::sign(&payload, &config.secret_key);
    let old_token = format!("{payload}|{sig}");
    assert!(!verify_csrf_token(&old_token, &config));
}

#[test]
fn two_tokens_differ() {
    let config = test_config();
    let t1 = generate_csrf_token(&config);
    let t2 = generate_csrf_token(&config);
    // Nonces must differ even when generated close together
    assert_ne!(t1.0, t2.0);
}

#[test]
fn constant_time_eq_works() {
    assert!(constant_time_eq("abcdef", "abcdef"));
    assert!(!constant_time_eq("abcdef", "abcdeg"));
    assert!(!constant_time_eq("abc", "abcd"));
}
