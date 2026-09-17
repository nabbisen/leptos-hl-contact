// tests.rs — unit tests for the parent module.

use super::*;

fn test_config() -> FormTokenConfig {
    // `min_age` 0 unless a test is about the minimum age: a freshly issued
    // token is zero seconds old and would otherwise be TooYoung.
    FormTokenConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec()).with_min_age(0)
}

/// Build a token with a chosen timestamp, to test age without sleeping.
fn token_aged(secs_ago: u64, config: &FormTokenConfig) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let payload = format!(
        "{}|aabbccddeeff00112233445566778899",
        now.saturating_sub(secs_ago)
    );
    let sig = sign(&payload, &config.secret_key);
    format!("{payload}|{sig}")
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn default_config_has_a_two_second_minimum_age() {
    let c = FormTokenConfig::new(b"secret".to_vec());
    assert_eq!(c.min_age_secs, 2);
    assert_eq!(c.ttl_secs, 3600);
    assert_eq!(c.binding, Binding::None);
}

#[test]
fn builders_override_the_defaults() {
    let c = FormTokenConfig::new(b"secret".to_vec())
        .with_ttl(60)
        .with_min_age(5)
        .with_binding(Binding::Cookie);
    assert_eq!(c.ttl_secs, 60);
    assert_eq!(c.min_age_secs, 5);
    assert_eq!(c.binding, Binding::Cookie);
}

#[test]
fn debug_redacts_the_secret() {
    let c = FormTokenConfig::new(b"super-secret-value".to_vec());
    let s = format!("{c:?}");
    assert!(s.contains("<redacted>"), "{s}");
    assert!(!s.contains("super-secret-value"), "{s}");
}

// ---------------------------------------------------------------------------
// Round trip
// ---------------------------------------------------------------------------

#[test]
fn issued_token_verifies_once_old_enough() {
    let config = test_config();
    let token = issue_form_token(&config);
    assert_eq!(verify_form_token(&token.0, None, &config), Ok(()));
}

#[test]
fn two_tokens_differ() {
    let config = test_config();
    assert_ne!(issue_form_token(&config).0, issue_form_token(&config).0);
}

// ---------------------------------------------------------------------------
// Every error variant reachable from the public function
// ---------------------------------------------------------------------------

#[test]
fn malformed_tokens_are_rejected() {
    let config = test_config();
    for bad in ["", "not-a-token", "a|b", "abc|nonce|sig", "12x3|aabb|sig"] {
        assert_eq!(
            verify_form_token(bad, None, &config),
            Err(FormTokenError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn a_non_hex_nonce_is_malformed() {
    let config = test_config();
    let payload = "1000000|zzzz";
    let sig = sign(payload, &config.secret_key);
    assert_eq!(
        verify_form_token(&format!("{payload}|{sig}"), None, &config),
        Err(FormTokenError::Malformed)
    );
}

#[test]
fn a_tampered_signature_is_a_bad_signature() {
    let config = test_config();
    let token = issue_form_token(&config);
    let mut parts: Vec<&str> = token.0.splitn(3, '|').collect();
    // Replace the last hex digit with a *different* one.  Always writing `0`
    // left one signature in sixteen unchanged, and the test failed at random.
    let (head, last) = parts[2].split_at(parts[2].len() - 1);
    let flipped = format!("{head}{}", if last == "0" { "1" } else { "0" });
    parts[2] = &flipped;
    assert_eq!(
        verify_form_token(&parts.join("|"), None, &config),
        Err(FormTokenError::BadSignature)
    );
}

/// RFC 016 D3: a signature one hex digit off `hex::decode`s fine, so this
/// exercises `verify_slice` itself, not the decode step.
#[test]
fn a_signature_one_hex_digit_off_is_a_bad_signature() {
    let config = test_config();
    let token = issue_form_token(&config);
    let mut parts: Vec<&str> = token.0.splitn(3, '|').collect();
    let (head, last) = parts[2].split_at(parts[2].len() - 1);
    let flipped = format!("{head}{}", if last == "0" { "1" } else { "0" });
    parts[2] = &flipped;
    assert_eq!(
        verify_form_token(&parts.join("|"), None, &config),
        Err(FormTokenError::BadSignature)
    );
}

/// RFC 016 D3: a signature two hex digits (one byte) short fails
/// `verify_slice`'s length check.
#[test]
fn a_signature_one_byte_short_is_a_bad_signature() {
    let config = test_config();
    let token = issue_form_token(&config);
    let mut parts: Vec<&str> = token.0.splitn(3, '|').collect();
    let short = &parts[2][..parts[2].len() - 2];
    parts[2] = short;
    assert_eq!(
        verify_form_token(&parts.join("|"), None, &config),
        Err(FormTokenError::BadSignature)
    );
}

/// RFC 016 D3: a non-hex signature fails to decode before `verify_slice` is
/// ever called.
#[test]
fn a_non_hex_signature_is_a_bad_signature() {
    let config = test_config();
    let token = issue_form_token(&config);
    let mut parts: Vec<&str> = token.0.splitn(3, '|').collect();
    parts[2] = "zz-not-hex-zz";
    assert_eq!(
        verify_form_token(&parts.join("|"), None, &config),
        Err(FormTokenError::BadSignature)
    );
}

#[test]
fn a_token_signed_with_another_key_is_a_bad_signature() {
    let mine = test_config();
    let theirs = FormTokenConfig::new(b"a-completely-different-secret-key".to_vec());
    let token = issue_form_token(&theirs);
    assert_eq!(
        verify_form_token(&token.0, None, &mine),
        Err(FormTokenError::BadSignature)
    );
}

#[test]
fn an_old_token_is_expired() {
    let config = test_config().with_ttl(10);
    let token = token_aged(11, &config);
    assert_eq!(
        verify_form_token(&token, None, &config),
        Err(FormTokenError::Expired)
    );
}

#[test]
fn a_far_future_token_is_rejected() {
    let config = test_config();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let payload = format!("{}|aabbccddeeff00112233445566778899", now + 3600);
    let sig = sign(&payload, &config.secret_key);
    assert_eq!(
        verify_form_token(&format!("{payload}|{sig}"), None, &config),
        Err(FormTokenError::FromFuture)
    );
}

// ---------------------------------------------------------------------------
// Minimum age
// ---------------------------------------------------------------------------

/// One second old against a two-second minimum: a script's speed.
#[test]
fn a_token_younger_than_the_minimum_is_too_young() {
    let config = FormTokenConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec());
    assert_eq!(config.min_age_secs, 2);
    let token = token_aged(1, &config);
    assert_eq!(
        verify_form_token(&token, None, &config),
        Err(FormTokenError::TooYoung)
    );
}

/// Exactly at the minimum is accepted: the check is `age < min`.
#[test]
fn a_token_at_the_minimum_age_passes() {
    let config = FormTokenConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec());
    let token = token_aged(2, &config);
    assert_eq!(verify_form_token(&token, None, &config), Ok(()));
}

#[test]
fn a_zero_minimum_age_disables_the_check() {
    let config =
        FormTokenConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec()).with_min_age(0);
    let token = issue_form_token(&config);
    assert_eq!(verify_form_token(&token.0, None, &config), Ok(()));
}

/// The order matters: a forged token that is also too young must not reveal
/// that its signature was the real problem, and vice versa.  Age is checked
/// first, per RFC 004 D2.
#[test]
fn age_is_checked_before_the_signature() {
    let config = FormTokenConfig::new(b"test-secret-key-at-least-32-bytes-long".to_vec());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let forged = format!("{now}|aabbccddeeff00112233445566778899|{}", "0".repeat(64));
    assert_eq!(
        verify_form_token(&forged, None, &config),
        Err(FormTokenError::TooYoung)
    );
}

// ---------------------------------------------------------------------------
// Binding — configured but not exercised by this handoff
// ---------------------------------------------------------------------------

#[test]
fn binding_none_ignores_the_bound_value() {
    let config = test_config();
    let token = issue_form_token(&config);
    assert_eq!(
        verify_form_token(&token.0, Some("anything at all"), &config),
        Ok(())
    );
}

#[test]
fn binding_cookie_requires_a_matching_nonce() {
    let config = test_config().with_binding(Binding::Cookie);
    let token = issue_form_token(&config);
    let nonce = token.0.split('|').nth(1).unwrap().to_owned();

    assert_eq!(
        verify_form_token(&token.0, None, &config),
        Err(FormTokenError::BindingMissing)
    );
    assert_eq!(
        verify_form_token(&token.0, Some("not-the-nonce"), &config),
        Err(FormTokenError::BindingMismatch)
    );
    assert_eq!(verify_form_token(&token.0, Some(&nonce), &config), Ok(()));
}

// ---------------------------------------------------------------------------
// issue_for_request — the body of `issue_form_token_fn`
// ---------------------------------------------------------------------------

fn in_owner<T>(f: impl FnOnce() -> T) -> T {
    let owner = leptos::reactive::owner::Owner::new();
    owner.set();
    let out = f();
    drop(owner);
    out
}

fn bound_config() -> FormTokenContext {
    Arc::new(test_config().with_binding(Binding::Cookie))
}

fn nonce_of(token: &FormToken) -> &str {
    token.0.split('|').nth(1).unwrap()
}

/// Handoff 03 amendment: the fetch is a POST, so the page render's GET gate
/// does not protect it.  It must reuse the browser's nonce itself, or every
/// tab that fetches a token invalidates the others.
#[test]
fn a_fetched_token_reuses_the_browsers_nonce() {
    use leptos::context::provide_context;
    let cookie = "00eaaaa84b55005200eaaaa84b550052";

    let token = in_owner(|| {
        provide_context(bound_config());
        provide_context(FormTokenBinding(Some(cookie.into())));
        issue_for_request().expect("configured")
    });

    assert_eq!(nonce_of(&token), cookie);
    assert!(verify_form_token(&token.0, Some(cookie), &bound_config()).is_ok());
}

#[test]
fn a_fetched_token_mints_when_there_is_no_cookie() {
    use leptos::context::provide_context;

    let token = in_owner(|| {
        provide_context(bound_config());
        provide_context(FormTokenBinding(None));
        issue_for_request().expect("configured")
    });

    assert_eq!(nonce_of(&token).len(), 32);
}

#[test]
fn a_fetched_token_does_not_trust_an_unusable_cookie() {
    use leptos::context::provide_context;

    let token = in_owner(|| {
        provide_context(bound_config());
        provide_context(FormTokenBinding(Some("not-a-nonce".into())));
        issue_for_request().expect("configured")
    });

    assert_ne!(nonce_of(&token), "not-a-nonce");
    assert_eq!(nonce_of(&token).len(), 32);
}

/// Without binding the cookie means nothing, so it is not consulted — as in
/// `submit_contact`.
#[test]
fn a_fetched_token_ignores_the_cookie_without_binding() {
    use leptos::context::provide_context;
    let cookie = "00eaaaa84b55005200eaaaa84b550052";

    let token = in_owner(|| {
        provide_context::<FormTokenContext>(Arc::new(test_config()));
        provide_context(FormTokenBinding(Some(cookie.into())));
        issue_for_request().expect("configured")
    });

    assert_ne!(nonce_of(&token), cookie);
}

#[test]
fn a_fetched_token_is_handed_to_the_issuer() {
    use leptos::context::provide_context;
    use std::sync::Mutex;

    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let token = in_owner(|| {
        provide_context(bound_config());
        let seen = Arc::clone(&seen);
        provide_context(FormTokenIssuer(Arc::new(move |t| {
            seen.lock().unwrap().push(t.0.clone());
        })));
        issue_for_request().expect("configured")
    });

    assert_eq!(*seen.lock().unwrap(), vec![token.0]);
}

/// Fail closed, like `submit_contact`: no config, no token.
#[test]
fn a_fetched_token_is_refused_without_the_config() {
    let err = in_owner(issue_for_request).expect_err("not configured");
    assert!(err.to_string().contains("not_configured"), "{err}");
}

/// FR-VAL-06 (RFC 011 D4): the token's clock reads the present.
#[test]
fn now_unix_secs_matches_the_system_clock() {
    let system = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_secs();
    let token_clock = now_unix_secs();
    assert!(
        token_clock.abs_diff(system) <= 2,
        "token clock {token_clock}, system clock {system}"
    );
}
