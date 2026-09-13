//! Silent outcomes.

use std::sync::Arc;

use leptos_hl_contact::FilterDecision;

use crate::support::{FixedFilter, Harness, Setup};

/// T18, FR-ABUSE-09, FR-SUB-04: a honeypot hit and a filter's `SilentDrop`
/// are indistinguishable from a delivered submission — the same status,
/// `Location`, `serverfnredirect` header and body — with and without a
/// success page, in both request forms.  Only the genuine one is delivered.
///
/// This is the regression that shipped in 0.4.0: with a success page, the
/// honeypot answered without the redirect.
#[tokio::test]
async fn a_silent_outcome_is_indistinguishable_from_delivery() {
    for success_page in [false, true] {
        let delivered = Harness::new(Setup {
            success_page,
            ..Setup::default()
        });
        let honeypot = Harness::new(Setup {
            success_page,
            ..Setup::default()
        });
        let silent_drop = Harness::new(Setup {
            success_page,
            filter: Some(Arc::new(FixedFilter::new(
                FilterDecision::SilentDrop,
                "SilentDrop",
            ))),
            ..Setup::default()
        });

        let genuine = delivered.fields();
        let bot = honeypot.fields().set("website", "http://bot.example");
        let dropped = silent_drop.fields();

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
            let context = format!("success_page={success_page}, {form}");
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
            "success_page={success_page}"
        );
    }
}
