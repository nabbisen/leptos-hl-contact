// tests.rs — the challenge decision table, one test per row.

use std::sync::Arc;

use super::*;

/// A verifier that answers with a fixed result.
struct MockVerifier(Result<ChallengeOutcome, ChallengeError>);

impl ChallengeVerifier for MockVerifier {
    fn verify(&self, _token: &str) -> VerifyFuture<'_> {
        // `ChallengeError` is not `Clone`; rebuild the answer each call.
        let answer = match &self.0 {
            Ok(outcome) => Ok(outcome.clone()),
            Err(ChallengeError::Timeout) => Err(ChallengeError::Timeout),
            Err(ChallengeError::Unavailable(s)) => Err(ChallengeError::Unavailable(s.clone())),
            Err(ChallengeError::Misconfigured(s)) => Err(ChallengeError::Misconfigured(s.clone())),
        };
        Box::pin(async move { answer })
    }
}

fn ctx(
    answer: Result<ChallengeOutcome, ChallengeError>,
    policy: ChallengePolicy,
) -> ChallengeContext {
    ChallengeContext {
        verifier: Arc::new(MockVerifier(answer)),
        policy,
    }
}

fn passed() -> ChallengeOutcome {
    ChallengeOutcome {
        passed: true,
        ..Default::default()
    }
}

/// Rows 5–7 as `submit_contact` runs them: gate, verify, judge.
async fn decide(ctx: &ChallengeContext, token: &str) -> Result<(), ContactErrorCode> {
    match gate(Some(ctx), Some(token)) {
        Gate::Proceed => Ok(()),
        Gate::Reject(code) => Err(code),
        Gate::Verify => judge(ctx.verifier.verify(token).await, &ctx.policy),
    }
}

// ---- rows 1–4: gate ---------------------------------------------------------

#[test]
fn row_1_no_context_no_token_proceeds() {
    assert_eq!(gate(None, None), Gate::Proceed);
}

#[test]
fn row_2_no_context_with_token_is_not_configured() {
    assert_eq!(
        gate(None, Some("token")),
        Gate::Reject(ContactErrorCode::NotConfigured)
    );
}

#[test]
fn row_3_context_no_token_under_reject_is_required() {
    let c = ctx(Ok(passed()), ChallengePolicy::default());
    assert_eq!(
        gate(Some(&c), None),
        Gate::Reject(ContactErrorCode::ChallengeRequired)
    );
}

#[test]
fn row_4_context_no_token_under_accept_proceeds() {
    let c = ctx(
        Ok(passed()),
        ChallengePolicy {
            no_js: NoJsPolicy::AcceptWithHoneypotOnly,
            ..Default::default()
        },
    );
    assert_eq!(gate(Some(&c), None), Gate::Proceed);
}

// ---- rows 5–7: verify and judge -------------------------------------------

#[tokio::test]
async fn row_5_passed_proceeds() {
    let c = ctx(Ok(passed()), ChallengePolicy::default());
    assert_eq!(decide(&c, "token").await, Ok(()));
}

#[tokio::test]
async fn row_6_not_passed_fails() {
    let c = ctx(
        Ok(ChallengeOutcome {
            passed: false,
            error_codes: vec!["invalid-input-response".into()],
            ..Default::default()
        }),
        ChallengePolicy::default(),
    );
    assert_eq!(
        decide(&c, "token").await,
        Err(ContactErrorCode::ChallengeFailed)
    );
}

#[tokio::test]
async fn row_7_verifier_error_is_unavailable_for_every_variant() {
    for err in [
        ChallengeError::Timeout,
        ChallengeError::Unavailable("connection refused".into()),
        ChallengeError::Misconfigured("no secret".into()),
    ] {
        let c = ctx(Err(err), ChallengePolicy::default());
        assert_eq!(
            decide(&c, "token").await,
            Err(ContactErrorCode::ChallengeUnavailable)
        );
    }
}

// ---- edges -------------------------------------------------------------------

/// Vendors post their field even when the widget never produced a token.
#[test]
fn an_empty_or_blank_token_counts_as_absent() {
    assert_eq!(gate(None, Some("")), Gate::Proceed);
    assert_eq!(gate(None, Some("  \n")), Gate::Proceed);
    let c = ctx(Ok(passed()), ChallengePolicy::default());
    assert_eq!(
        gate(Some(&c), Some("")),
        Gate::Reject(ContactErrorCode::ChallengeRequired)
    );
}

#[test]
fn a_score_exactly_at_the_minimum_passes() {
    let outcome = ChallengeOutcome {
        passed: true,
        score: Some(0.5),
        ..Default::default()
    };
    assert_eq!(judge(Ok(outcome), &ChallengePolicy::default()), Ok(()));
}

#[test]
fn a_score_below_the_minimum_fails() {
    let outcome = ChallengeOutcome {
        passed: true,
        score: Some(0.49),
        ..Default::default()
    };
    assert_eq!(
        judge(Ok(outcome), &ChallengePolicy::default()),
        Err(ContactErrorCode::ChallengeFailed)
    );
}

/// `NaN < 0.5` is false, so a naive comparison would let it through.
#[test]
fn a_nan_score_fails() {
    let outcome = ChallengeOutcome {
        passed: true,
        score: Some(f32::NAN),
        ..Default::default()
    };
    assert_eq!(
        judge(Ok(outcome), &ChallengePolicy::default()),
        Err(ContactErrorCode::ChallengeFailed)
    );
}

#[test]
fn an_expected_action_mismatch_fails() {
    let policy = ChallengePolicy {
        expected_action: Some("contact".into()),
        ..Default::default()
    };
    for action in [Some("login".to_owned()), None] {
        let outcome = ChallengeOutcome {
            passed: true,
            action,
            ..Default::default()
        };
        assert_eq!(
            judge(Ok(outcome), &policy),
            Err(ContactErrorCode::ChallengeFailed)
        );
    }
}

#[test]
fn an_expected_action_match_passes() {
    let policy = ChallengePolicy {
        expected_action: Some("contact".into()),
        ..Default::default()
    };
    let outcome = ChallengeOutcome {
        passed: true,
        action: Some("contact".into()),
        ..Default::default()
    };
    assert_eq!(judge(Ok(outcome), &policy), Ok(()));
}

#[test]
fn an_unset_expected_action_ignores_the_action() {
    let outcome = ChallengeOutcome {
        passed: true,
        action: Some("anything".into()),
        ..Default::default()
    };
    assert_eq!(judge(Ok(outcome), &ChallengePolicy::default()), Ok(()));
}

#[test]
fn the_policy_defaults_are_the_documented_ones() {
    let p = ChallengePolicy::default();
    assert_eq!(p.no_js, NoJsPolicy::Reject);
    assert_eq!(p.min_score, 0.5);
    assert_eq!(p.expected_action, None);
}
