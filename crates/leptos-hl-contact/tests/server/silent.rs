//! Silent outcomes.

use std::sync::Arc;

use leptos_hl_contact::FilterDecision;

use crate::support::{Fields, FixedFilter, Harness, Setup, TokenMode};

fn harness(token: TokenMode, success_page: bool, silent_drop: bool) -> Harness {
    Harness::new(Setup {
        token,
        success_page,
        filter: silent_drop
            .then(|| Arc::new(FixedFilter::new(FilterDecision::SilentDrop, "SilentDrop")) as _),
        ..Setup::default()
    })
}

/// A valid submission for `h`.  When bound, the page is rendered first and
/// its token is sent with its cookie.
async fn submission(h: &Harness, token: TokenMode) -> Fields {
    if token != TokenMode::Bound {
        return h.fields();
    }
    let page = h.render("/contact", None).await;
    let cookie = page
        .set_cookies
        .iter()
        .find(|c| c.contains("hl_contact_ft="))
        .and_then(|c| c.split(';').next())
        .expect("a binding cookie")
        .to_owned();
    Fields::valid(&page.token().expect("token")).with_cookie(&cookie)
}

/// T18, FR-ABUSE-09, FR-SUB-04: a honeypot hit and a filter's `SilentDrop`
/// are indistinguishable from a delivered submission — the same status, body
/// and every header, `Location` and `serverfnredirect` included — with and
/// without a success page, with and without cookie binding, in both request
/// forms.  Only the genuine one is delivered.
///
/// This is the regression that shipped in 0.4.0: with a success page, the
/// honeypot answered without the redirect.
#[tokio::test]
async fn a_silent_outcome_is_indistinguishable_from_delivery() {
    for token in [TokenMode::Plain, TokenMode::Bound] {
        for success_page in [false, true] {
            let delivered = harness(token, success_page, false);
            let honeypot = harness(token, success_page, false);
            let silent_drop = harness(token, success_page, true);

            let genuine = submission(&delivered, token).await;
            let bot = submission(&honeypot, token)
                .await
                .set("website", "http://bot.example");
            let dropped = submission(&silent_drop, token).await;

            for form in ["no-JS", "fetch"] {
                let (a, b, c) = if form == "no-JS" {
                    (
                        delivered.submit_nojs(&genuine).await,
                        honeypot.submit_nojs(&bot).await,
                        silent_drop.submit_nojs(&dropped).await,
                    )
                } else {
                    (
                        delivered.submit_fetch(&genuine).await,
                        honeypot.submit_fetch(&bot).await,
                        silent_drop.submit_fetch(&dropped).await,
                    )
                };
                let context = format!("{token:?}, success_page={success_page}, {form}");
                assert_eq!(
                    a.fingerprint(),
                    b.fingerprint(),
                    "honeypot vs delivery, {context}"
                );
                assert_eq!(
                    a.fingerprint(),
                    c.fingerprint(),
                    "SilentDrop vs delivery, {context}"
                );
                if success_page {
                    assert_eq!(a.location().as_deref(), Some("/thanks"), "{context}");
                }
            }

            assert_eq!(
                [
                    delivered.deliveries(),
                    honeypot.deliveries(),
                    silent_drop.deliveries()
                ],
                [2, 0, 0],
                "{token:?}, success_page={success_page}"
            );
        }
    }
}
