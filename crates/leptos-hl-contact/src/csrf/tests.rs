// tests.rs — unit tests for the parent module.
//
// These exist to prove the 0.4 names still compile and behave, which is the
// whole point of the alias module.  `#[allow(deprecated)]` is deliberate:
// deprecating the aliases is what this module is for.
#![allow(deprecated)]

use super::*;

#[test]
fn deprecated_aliases_still_name_the_new_types() {
    let config: CsrfConfig =
        FormTokenConfig::new(b"secret-key-at-least-32-bytes-long".to_vec()).with_min_age(0);
    let token: CsrfToken = generate_csrf_token(&config);
    let _ctx: CsrfConfigContext = std::sync::Arc::new(config.clone());

    assert!(!token.0.is_empty());
}

#[test]
fn deprecated_round_trip_is_true() {
    // `min_age` 0, or a freshly issued token would be rejected as too young
    // and this alias cannot report why.
    let config =
        FormTokenConfig::new(b"secret-key-at-least-32-bytes-long".to_vec()).with_min_age(0);
    let token = generate_csrf_token(&config);
    assert!(verify_csrf_token(&token.0, &config));
}

#[test]
fn deprecated_verify_is_false_for_a_tampered_token() {
    let config =
        FormTokenConfig::new(b"secret-key-at-least-32-bytes-long".to_vec()).with_min_age(0);
    let token = generate_csrf_token(&config);
    let tampered = format!("{}x", token.0);
    assert!(!verify_csrf_token(&tampered, &config));
}
