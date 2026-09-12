// tests.rs — unit tests for the parent module.

use super::*;
use crate::delivery::noop::NoopDelivery;
use std::sync::Arc;

#[test]
fn delivery_context_fn_is_clone() {
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);
    // Should be cloneable (required by both Axum handler sites).
    let _ctx2 = ctx.clone();
}

// ---------------------------------------------------------------------------
// Cookie binding — compiled only with the token itself.
// ---------------------------------------------------------------------------

#[cfg(feature = "form-token")]
mod cookie_binding {
    use crate::axum_helpers::*;

    #[test]
    fn cookie_value_finds_the_named_cookie() {
        let header = "session=abc; hl_contact_ft=deadbeef; theme=dark";
        assert_eq!(
            cookie_value(header, "hl_contact_ft"),
            Some("deadbeef".into())
        );
    }

    #[test]
    fn cookie_value_handles_a_single_cookie_and_odd_spacing() {
        assert_eq!(
            cookie_value("hl_contact_ft=deadbeef", "hl_contact_ft"),
            Some("deadbeef".into())
        );
        assert_eq!(
            cookie_value("  hl_contact_ft =  deadbeef  ; x=1", "hl_contact_ft"),
            Some("deadbeef".into())
        );
    }

    /// A prefix match would let `hl_contact_ft2` satisfy a lookup for
    /// `hl_contact_ft`, so an attacker who can set any cookie could supply the
    /// binding value.  The name must match whole.
    #[test]
    fn cookie_value_rejects_prefix_matches() {
        let header = "hl_contact_ft2=attacker; other=1";
        assert_eq!(cookie_value(header, "hl_contact_ft"), None);

        let header = "xhl_contact_ft=attacker";
        assert_eq!(cookie_value(header, "hl_contact_ft"), None);
    }

    #[test]
    fn cookie_value_is_none_when_absent_or_malformed() {
        assert_eq!(
            cookie_value("session=abc; theme=dark", "hl_contact_ft"),
            None
        );
        assert_eq!(cookie_value("", "hl_contact_ft"), None);
        assert_eq!(cookie_value("no-equals-sign", "hl_contact_ft"), None);
    }

    #[test]
    fn cookie_value_takes_the_first_of_duplicates() {
        let header = "hl_contact_ft=first; hl_contact_ft=second";
        assert_eq!(cookie_value(header, "hl_contact_ft"), Some("first".into()));
    }

    // ---------------------------------------------------------------------------
    // Set-Cookie construction
    // ---------------------------------------------------------------------------

    #[test]
    fn set_cookie_value_has_every_required_attribute() {
        let c = FormTokenCookie::default();
        assert_eq!(
            set_cookie_value("deadbeef", &c, 3600),
            "__Host-hl_contact_ft=deadbeef; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600; Secure"
        );
    }

    #[test]
    fn set_cookie_value_omits_secure_when_asked() {
        let c = FormTokenCookie {
            secure: false,
            ..Default::default()
        };
        let v = set_cookie_value("deadbeef", &c, 60);
        assert_eq!(
            v,
            "hl_contact_ft=deadbeef; HttpOnly; SameSite=Lax; Path=/; Max-Age=60"
        );
        assert!(!v.contains("Secure"));
    }

    /// `HttpOnly` keeps the value out of scripts and `SameSite=Lax` keeps the
    /// browser from sending it on a cross-site POST.  Either one missing would
    /// void the CSRF property, so they are asserted directly.
    #[test]
    fn set_cookie_value_is_never_script_readable_or_cross_site() {
        for secure in [true, false] {
            let c = FormTokenCookie {
                secure,
                ..Default::default()
            };
            let v = set_cookie_value("deadbeef", &c, 3600);
            assert!(v.contains("HttpOnly"), "{v}");
            assert!(v.contains("SameSite=Lax"), "{v}");
            assert!(!v.contains("SameSite=None"), "{v}");
        }
    }

    #[test]
    fn set_cookie_value_honours_a_custom_name_and_path() {
        let c = FormTokenCookie {
            name: "custom".into(),
            secure: true,
            path: "/contact".into(),
        };
        assert_eq!(
            set_cookie_value("nonce", &c, 120),
            "custom=nonce; HttpOnly; SameSite=Lax; Path=/contact; Max-Age=120; Secure"
        );
    }

    #[test]
    fn cookie_defaults_are_the_documented_ones() {
        let c = FormTokenCookie::default();
        assert_eq!(c.name, "hl_contact_ft");
        assert!(c.secure);
        assert_eq!(c.path, "/");
    }

    // ---------------------------------------------------------------------------
    // The nonce is what goes in the cookie — never the token or the secret
    // ---------------------------------------------------------------------------

    #[test]
    fn token_nonce_is_the_middle_segment() {
        assert_eq!(token_nonce("1700000000|abc123|deadbeef"), Some("abc123"));
        assert_eq!(token_nonce("no-pipes"), None);
    }

    #[test]
    fn the_cookie_carries_only_the_nonce() {
        use crate::form_token::{FormTokenConfig, issue_form_token};

        let config = FormTokenConfig::new(b"a-secret-key-at-least-32-bytes-long".to_vec());
        let token = issue_form_token(&config);
        let nonce = token_nonce(&token.0).unwrap();

        let v = set_cookie_value(nonce, &FormTokenCookie::default(), config.ttl_secs);
        assert!(v.contains(nonce));
        assert!(
            !v.contains(&token.0),
            "the whole token must not be in the cookie"
        );
        assert!(
            !v.contains("a-secret-key"),
            "the secret must never reach the cookie"
        );
    }

    // ---------------------------------------------------------------------------
    // GET-only issuance
    // ---------------------------------------------------------------------------

    /// Build a `Parts` with the given method, as a request would carry.
    fn parts_with(method: axum::http::Method, cookie: Option<&str>) -> axum::http::request::Parts {
        let mut b = axum::http::Request::builder().method(method).uri("/");
        if let Some(c) = cookie {
            b = b.header(axum::http::header::COOKIE, c);
        }
        b.body(()).unwrap().into_parts().0
    }

    fn in_scope<T>(f: impl FnOnce() -> T) -> T {
        let owner = leptos::reactive::owner::Owner::new();
        owner.set();
        let out = f();
        drop(owner);
        out
    }

    /// One closure serves page renders and server functions alike (RFC 007), so
    /// issuing on a POST would overwrite the cookie the submitted form is bound
    /// to — turning every second submission into a `BindingMismatch`.
    #[test]
    fn issuing_is_a_no_op_for_a_post() {
        use crate::form_token::{FormToken, FormTokenConfig};
        use leptos::context::{provide_context, use_context};

        let config: FormTokenContext = Arc::new(FormTokenConfig::new(
            b"a-secret-key-at-least-32-bytes".to_vec(),
        ));

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::POST, None));
            provide_form_token_with_cookie(&config, &FormTokenCookie::default());
            assert!(
                use_context::<FormToken>().is_none(),
                "a POST must not issue a token"
            );
        });
    }

    #[test]
    fn issuing_happens_for_a_get() {
        use crate::form_token::{FormToken, FormTokenConfig};
        use leptos::context::{provide_context, use_context};

        let config: FormTokenContext = Arc::new(FormTokenConfig::new(
            b"a-secret-key-at-least-32-bytes".to_vec(),
        ));

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::GET, None));
            provide_form_token_with_cookie(&config, &FormTokenCookie::default());
            let token = use_context::<FormToken>().expect("a GET issues a token");
            assert_eq!(token.0.split('|').count(), 3);
        });
    }

    // -----------------------------------------------------------------------
    // The `__Host-` prefix (review C2)
    // -----------------------------------------------------------------------

    /// At the defaults the cookie is `Secure` on `/` with no `Domain`, which is
    /// exactly what `__Host-` requires, so a sibling subdomain cannot toss one.
    #[test]
    fn the_prefix_is_applied_at_the_defaults() {
        let cookie = FormTokenCookie::default();
        assert_eq!(effective_name(&cookie), "__Host-hl_contact_ft");
        assert!(set_cookie_value("ab", &cookie, 60).starts_with("__Host-hl_contact_ft=ab;"));
    }

    /// A browser refuses a `__Host-` cookie without `Secure`, so the local-HTTP
    /// override must keep the bare name or the cookie is silently dropped.
    #[test]
    fn the_prefix_is_not_applied_without_secure() {
        let cookie = FormTokenCookie {
            secure: false,
            ..Default::default()
        };
        assert_eq!(effective_name(&cookie), "hl_contact_ft");
    }

    #[test]
    fn the_prefix_is_not_applied_off_the_root_path() {
        let cookie = FormTokenCookie {
            path: "/contact".into(),
            ..Default::default()
        };
        assert_eq!(effective_name(&cookie), "hl_contact_ft");
    }

    #[test]
    fn a_name_that_already_has_the_prefix_is_not_doubled() {
        let cookie = FormTokenCookie {
            name: "__Host-custom".into(),
            ..Default::default()
        };
        assert_eq!(effective_name(&cookie), "__Host-custom");
    }

    /// Write and read must agree: a cookie set under the prefixed name is the
    /// one the binding reads back.
    #[test]
    fn the_binding_is_read_under_the_prefixed_name() {
        use crate::form_token::FormTokenBinding;
        use leptos::context::{provide_context, use_context};

        let cookie = FormTokenCookie::default();
        let written = set_cookie_value("deadbeef", &cookie, 60);
        let pair = written.split(';').next().unwrap().to_owned();

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::POST, Some(&pair)));
            provide_form_token_binding(&cookie);
            assert_eq!(
                use_context::<FormTokenBinding>().unwrap().0.as_deref(),
                Some("deadbeef")
            );
        });
    }

    /// The attack C2 closes: a subdomain can plant the *bare* name with
    /// `Domain=.example.com`, but not the prefixed one.  The bare name must
    /// therefore be ignored whenever the prefix is in force.
    #[test]
    fn a_bare_name_is_ignored_while_the_prefix_is_in_force() {
        use crate::form_token::FormTokenBinding;
        use leptos::context::{provide_context, use_context};

        in_scope(|| {
            provide_context(parts_with(
                axum::http::Method::POST,
                Some("hl_contact_ft=00eaaaa84b55005200eaaaa84b550052"),
            ));
            provide_form_token_binding(&FormTokenCookie::default());
            assert_eq!(use_context::<FormTokenBinding>().unwrap().0, None);
        });
    }

    // -----------------------------------------------------------------------
    // Nonce stability (RFC 004 D3, as amended)
    // -----------------------------------------------------------------------

    fn test_config() -> FormTokenContext {
        Arc::new(crate::form_token::FormTokenConfig::new(
            b"a-secret-key-at-least-32-bytes".to_vec(),
        ))
    }

    /// Issue a token the way a page render does, for a request carrying
    /// `cookie`, and return the token's nonce.
    fn issued_nonce(config: &FormTokenContext, cookie: Option<&str>) -> String {
        use leptos::context::{provide_context, use_context};

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::GET, cookie));
            provide_form_token_with_cookie(config, &FormTokenCookie::default());
            let token = use_context::<crate::form_token::FormToken>().expect("a GET issues");
            token_nonce(&token.0).expect("three segments").to_owned()
        })
    }

    /// The cookie identifies the browser, so a render that already has one
    /// signs *that* nonce.  Minting a fresh one would overwrite the cookie and
    /// invalidate every form the visitor has open elsewhere.
    #[test]
    fn a_usable_cookie_is_reused_as_the_nonce() {
        let config = test_config();
        let existing = "00eaaaa84b55005200eaaaa84b550052";

        assert_eq!(
            issued_nonce(&config, Some(&format!("__Host-hl_contact_ft={existing}"))),
            existing
        );
    }

    /// The header goes out on every render even when the value is unchanged, so
    /// `Max-Age` is refreshed while the visitor is active.
    #[test]
    fn the_cookie_is_re_sent_when_it_is_reused() {
        let existing = "00eaaaa84b55005200eaaaa84b550052";
        let value = set_cookie_value(existing, &FormTokenCookie::default(), 3600);

        assert!(value.starts_with(&format!("__Host-hl_contact_ft={existing};")));
        assert!(value.contains("Max-Age=3600"));
    }

    #[test]
    fn no_cookie_mints_a_new_nonce() {
        let config = test_config();
        let first = issued_nonce(&config, None);
        let second = issued_nonce(&config, None);

        assert_eq!(first.len(), 32);
        assert_ne!(
            first, second,
            "without a cookie to reuse, each render mints its own nonce"
        );
    }

    /// A truncated, over-long or non-hex value is never signed into a token:
    /// the helper mints instead of trusting whatever arrived.
    #[test]
    fn an_unusable_cookie_value_is_not_trusted() {
        let config = test_config();

        for bad in [
            "",                                  // empty
            "00eaaaa84b550052",                  // half length
            "00eaaaa84b55005200eaaaa84b5500522", // one over
            "zzeaaaa84b55005200eaaaa84b550052",  // right length, not hex
        ] {
            let nonce = issued_nonce(&config, Some(&format!("__Host-hl_contact_ft={bad}")));
            assert_ne!(nonce, bad, "{bad:?} must not become the nonce");
            assert_eq!(nonce.len(), 32);
        }
    }

    /// The defect this correction fixes: two renders against one browser used
    /// to yield two nonces, so only the newest form could submit.
    #[test]
    fn two_renders_for_one_browser_share_a_nonce() {
        let config = test_config();

        // First visit: nothing to reuse.
        let first = issued_nonce(&config, None);
        // The browser now sends it back, on this page and on every other.
        let header = format!("__Host-hl_contact_ft={first}");
        let second = issued_nonce(&config, Some(&header));
        let third = issued_nonce(&config, Some(&header));

        assert_eq!(first, second);
        assert_eq!(first, third);
    }

    /// Reuse must not make the *token* stale: the timestamp is re-stamped, so
    /// the TTL and the minimum age still count from this render.
    #[test]
    fn a_reused_nonce_still_yields_a_fresh_token() {
        use crate::form_token::issue_form_token_with_nonce;

        let config = crate::form_token::FormTokenConfig::new(b"a-secret-key".to_vec());
        let nonce = "00eaaaa84b55005200eaaaa84b550052";

        let token = issue_form_token_with_nonce(&config, nonce).expect("32 hex chars");
        let issued: u64 = token.0.split('|').next().unwrap().parse().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert!(now.saturating_sub(issued) <= 1, "stamped at issue time");
        assert!(issue_form_token_with_nonce(&config, "too-short").is_none());
    }

    // -----------------------------------------------------------------------
    // The issuer — the cookie for a token fetched from the browser
    // -----------------------------------------------------------------------

    /// The whole path a `POST /api/form_token` takes through the one context
    /// closure: the page-render helper does nothing (not a GET), the binding
    /// is read under the prefixed name, the token reuses that nonce, and the
    /// issuer re-sends the same prefixed cookie.
    #[test]
    fn a_fetched_token_reuses_the_nonce_and_re_sends_the_cookie() {
        use crate::form_token::{Binding, FormToken, FormTokenConfig, issue_for_request};
        use leptos::context::{provide_context, use_context};

        let config: FormTokenContext = Arc::new(
            FormTokenConfig::new(b"a-secret-key-at-least-32-bytes".to_vec())
                .with_binding(Binding::Cookie),
        );
        let cookie = FormTokenCookie::default();
        let existing = "00eaaaa84b55005200eaaaa84b550052";
        let res = leptos_axum::ResponseOptions::default();

        let token = in_scope(|| {
            provide_context(parts_with(
                axum::http::Method::POST,
                Some(&format!("__Host-hl_contact_ft={existing}")),
            ));
            provide_context(res.clone());
            provide_context(Arc::clone(&config));
            provide_form_token_with_cookie(&config, &cookie);
            provide_form_token_binding(&cookie);
            provide_form_token_issuer(&cookie);
            assert!(
                use_context::<FormToken>().is_none(),
                "the page-render helper must stay out of a POST"
            );
            issue_for_request().expect("configured")
        });

        assert_eq!(token_nonce(&token.0), Some(existing));
        let headers = res.0.read().unwrap().headers.clone();
        let set: Vec<_> = headers
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap().to_owned())
            .collect();
        assert_eq!(set, vec![set_cookie_value(existing, &cookie, 3600)]);
    }

    /// Without a request in context there is nothing to decide on, so nothing is
    /// issued rather than a token being handed out blindly.
    #[test]
    fn issuing_is_a_no_op_without_request_parts() {
        use crate::form_token::{FormToken, FormTokenConfig};
        use leptos::context::use_context;

        let config: FormTokenContext = Arc::new(FormTokenConfig::new(
            b"a-secret-key-at-least-32-bytes".to_vec(),
        ));

        in_scope(|| {
            provide_form_token_with_cookie(&config, &FormTokenCookie::default());
            assert!(use_context::<FormToken>().is_none());
        });
    }

    // ---------------------------------------------------------------------------
    // Binding from the request
    // ---------------------------------------------------------------------------

    #[test]
    fn binding_is_read_from_the_cookie_header() {
        use crate::form_token::FormTokenBinding;
        use leptos::context::{provide_context, use_context};

        in_scope(|| {
            provide_context(parts_with(
                axum::http::Method::POST,
                Some("a=1; __Host-hl_contact_ft=deadbeef; b=2"),
            ));
            provide_form_token_binding(&FormTokenCookie::default());
            let b = use_context::<FormTokenBinding>().expect("binding provided");
            assert_eq!(b.0.as_deref(), Some("deadbeef"));
        });
    }

    /// An absent cookie, an absent header and an absent request all mean the same
    /// thing: nothing arrived, so `Binding::Cookie` rejects the submission.
    #[test]
    fn binding_is_none_when_nothing_arrived() {
        use crate::form_token::FormTokenBinding;
        use leptos::context::{provide_context, use_context};

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::POST, Some("other=1")));
            provide_form_token_binding(&FormTokenCookie::default());
            assert!(use_context::<FormTokenBinding>().unwrap().0.is_none());
        });

        in_scope(|| {
            provide_context(parts_with(axum::http::Method::POST, None));
            provide_form_token_binding(&FormTokenCookie::default());
            assert!(use_context::<FormTokenBinding>().unwrap().0.is_none());
        });

        in_scope(|| {
            provide_form_token_binding(&FormTokenCookie::default());
            assert!(use_context::<FormTokenBinding>().unwrap().0.is_none());
        });
    }
}
