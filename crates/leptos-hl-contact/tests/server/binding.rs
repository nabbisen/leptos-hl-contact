//! The form token bound to a `__Host-` cookie.

use crate::support::{Fields, Harness, Setup, TokenMode, nonce_of};

fn bound() -> Harness {
    Harness::new(Setup {
        token: TokenMode::Bound,
        ..Setup::default()
    })
}

/// The binding cookie's `name=value` pair from a `Set-Cookie` list.
fn cookie_pair(set_cookies: &[String]) -> String {
    set_cookies
        .iter()
        .find(|c| c.contains("hl_contact_ft="))
        .and_then(|c| c.split(';').next())
        .expect("a binding cookie")
        .to_owned()
}

/// T17, FR-ABUSE-02: at the defaults the cookie is `__Host-` prefixed,
/// `HttpOnly`, `SameSite=Lax`, `Secure` on `/`, and carries the token's nonce.
#[tokio::test]
async fn binding_uses_the_host_prefix_at_the_defaults() {
    let h = bound();
    let page = h.render("/contact", None).await;
    let token = page.token().expect("a rendered token");

    let cookie = page
        .set_cookies
        .iter()
        .find(|c| c.contains("hl_contact_ft="))
        .expect("Set-Cookie");
    assert!(
        cookie.starts_with(&format!("__Host-hl_contact_ft={};", nonce_of(&token))),
        "{cookie}"
    );
    for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Secure"] {
        assert!(cookie.contains(attribute), "{attribute} in {cookie}");
    }
}

/// T17: the token is accepted with its cookie, and rejected without it or
/// with a cookie carrying another nonce — in both request forms.
#[tokio::test]
async fn binding_requires_the_matching_cookie() {
    let h = bound();
    let page = h.render("/contact", None).await;
    let token = page.token().expect("token");
    let cookie = cookie_pair(&page.set_cookies);
    let expired = "Your session token expired. Please reload the page and try again.";

    let with_cookie = Fields::valid(&token).with_cookie(&cookie);
    let accepted = h.submit_fetch(&with_cookie).await;
    assert!(accepted.status.is_success(), "{}", accepted.body);
    assert!(h.submit_nojs(&with_cookie).await.is_nojs_success());
    assert_eq!(h.deliveries(), 2);

    let missing = Fields::valid(&token);
    let mismatched =
        Fields::valid(&token).with_cookie("__Host-hl_contact_ft=00000000000000000000000000000000");
    for (case, fields) in [("missing", missing), ("mismatched", mismatched)] {
        let fetch = h.submit_fetch(&fields).await;
        assert_eq!(
            fetch.contact_error().as_deref(),
            Some("token_invalid"),
            "{case}, fetch"
        );
        let nojs = h.submit_nojs(&fields).await;
        assert!(nojs.is_nojs_error(), "{case}, no-JS");
        assert_eq!(
            h.follow(&nojs).await.banner().as_deref(),
            Some(expired),
            "{case}, no-JS"
        );
    }
    assert_eq!(h.deliveries(), 2);
}

/// T17: a sibling subdomain can plant the *bare* cookie name but not a
/// `__Host-` one.  With the prefix in force, the bare name is ignored even
/// when it carries the right nonce — in both request forms.
#[tokio::test]
async fn binding_rejects_a_tossed_bare_cookie() {
    let h = bound();
    let page = h.render("/contact", None).await;
    let token = page.token().expect("token");

    let tossed = Fields::valid(&token).with_cookie(&format!("hl_contact_ft={}", nonce_of(&token)));

    let fetch = h.submit_fetch(&tossed).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("token_invalid"));

    let nojs = h.submit_nojs(&tossed).await;
    assert!(nojs.is_nojs_error(), "{:?}", nojs.location());
    assert_eq!(
        h.follow(&nojs).await.banner().as_deref(),
        Some("Your session token expired. Please reload the page and try again.")
    );
    assert_eq!(h.deliveries(), 0);
}

/// RFC 004 D3 as amended, T17: the nonce is per browser, not per render.  A
/// second render for the same browser reuses the cookie's nonce and re-sends
/// the cookie, so the token from the **first** render still submits.
#[tokio::test]
async fn binding_reuses_the_browser_nonce_across_renders() {
    let h = bound();
    let first = h.render("/contact", None).await;
    let first_token = first.token().expect("token");
    let cookie = cookie_pair(&first.set_cookies);

    let second = h.render("/contact", Some(&cookie)).await;
    let second_token = second.token().expect("token");
    assert_eq!(
        nonce_of(&second_token),
        nonce_of(&first_token),
        "same nonce"
    );
    assert_eq!(
        cookie_pair(&second.set_cookies),
        cookie,
        "cookie re-sent unchanged"
    );

    let reply = h
        .submit_fetch(&Fields::valid(&first_token).with_cookie(&cookie))
        .await;
    assert!(
        reply.status.is_success(),
        "the first render's token: {}",
        reply.body
    );
    assert_eq!(h.deliveries(), 1);
}

/// RFC 004 handoff 03 amendment: the token endpoint reuses the browser's nonce
/// too, and re-sends the same `__Host-` cookie.
#[tokio::test]
async fn the_token_endpoint_reuses_the_nonce() {
    let h = bound();
    let page = h.render("/contact", None).await;
    let rendered = page.token().expect("token");
    let cookie = cookie_pair(&page.set_cookies);

    let reply = h.fetch_token(Some(&cookie)).await;
    assert!(reply.status.is_success(), "{}", reply.body);
    let fetched = reply.body.trim_matches('"').to_owned();
    assert_eq!(nonce_of(&fetched), nonce_of(&rendered));
    assert_eq!(cookie_pair(&reply.set_cookies()), cookie);

    let submitted = h
        .submit_fetch(&Fields::valid(&rendered).with_cookie(&cookie))
        .await;
    assert!(submitted.status.is_success(), "{}", submitted.body);
}
