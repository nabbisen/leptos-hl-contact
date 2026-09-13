# Handoff 01 — Server side: trait, decision table, arguments

**RFC.** [005](../../done/005-challenge-providers.md), D2, D3, amendments.
**Requirements.** FR-ABUSE-10, FR-ABUSE-11, FR-ABUSE-12.

## Purpose

Let `submit_contact` verify a vendor token through a trait, fail-closed,
with the decision table fully covered by tests before any HTTP code
exists.

## Change scope

New `src/challenge.rs` + `src/challenge/tests.rs`; `src/config.rs`
(`NoJsPolicy` only, shared with the component); `src/error.rs`
(codes); `src/config.rs` labels; `src/server.rs`; `src/lib.rs`.

## Explicit non-change scope

No HTTP client, no widget markup, no features in `Cargo.toml`.  Order of
existing checks unchanged.

## Required implementation

1. **`config.rs`.**  `pub enum NoJsPolicy { Reject, AcceptWithHoneypotOnly }`,
   `Default = Reject`, with rustdoc stating the weakness sentence from
   RFC D3 verbatim.
2. **`challenge.rs`** (compiled under `ssr`; no feature flag):

   ```rust
   pub struct ChallengeOutcome { pub passed: bool, pub score: Option<f32>, pub action: Option<String>, pub error_codes: Vec<String> }
   #[derive(Debug, thiserror::Error)] pub enum ChallengeError { #[error("timeout")] Timeout, #[error("unavailable: {0}")] Unavailable(String), #[error("misconfigured: {0}")] Misconfigured(String) }
   pub trait ChallengeVerifier: Send + Sync + 'static {
       fn verify(&self, token: &str) -> Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + '_>>;
   }
   pub struct ChallengePolicy { pub no_js: NoJsPolicy, pub min_score: f32 /* 0.5 */, pub expected_action: Option<String> }
   pub struct ChallengeContext { pub verifier: Arc<dyn ChallengeVerifier>, pub policy: ChallengePolicy }
   ```

   Pure decision functions, `pub(crate)`, each a single `match`:

   ```rust
   pub(crate) enum Gate { Proceed, Reject(ContactErrorCode), Verify }
   pub(crate) fn gate(ctx: Option<&ChallengeContext>, token: Option<&str>) -> Gate;
   pub(crate) fn judge(result: Result<ChallengeOutcome, ChallengeError>, policy: &ChallengePolicy) -> Result<(), ContactErrorCode>;
   ```

   `gate` implements rows 1–4 of the RFC table (empty string counts as
   absent; row 2 returns `Reject(NotConfigured)`).  `judge` implements
   rows 5–7: `Err(_)` → `ChallengeUnavailable`; `!passed` →
   `ChallengeFailed`; `score` present and `< min_score` → `ChallengeFailed`;
   `expected_action` set and `action != expected` → `ChallengeFailed`;
   otherwise `Ok`.
3. **`error.rs`.**  `ContactErrorCode::{ChallengeRequired, ChallengeFailed, ChallengeUnavailable}`
   (`challenge_required`, `challenge_failed`, `challenge_unavailable`).
4. **Labels.**  `ContactErrorLabels`: `challenge_required`,
   `challenge_failed`, `challenge_unavailable`, `challenge_requires_js`
   with the English defaults from RFC D3.
5. **`server.rs`.**  Add the three renamed arguments exactly as in RFC
   D2.  After the server-policy step and before delivery:

   ```rust
   let challenge_token = [cf_turnstile_response, h_captcha_response, g_recaptcha_response]
       .into_iter().flatten().find(|t| !t.trim().is_empty());
   let ctx = use_context::<ChallengeContext>();
   match challenge::gate(ctx.as_ref(), challenge_token.as_deref()) {
       Gate::Proceed => {}
       Gate::Reject(code) => { /* log per table; return Args or ServerError per code class */ }
       Gate::Verify => {
           let ctx = ctx.expect("gate returned Verify only with a context");
           let result = ctx.verifier.verify(challenge_token.as_deref().unwrap()).await;
           if let Err(code) = challenge::judge(result, &ctx.policy) { /* log; return */ }
       }
   }
   ```

   Logging: row 2 `error!("challenge token received but no ChallengeContext is provided")`;
   row 4 `info!("challenge skipped: no token, policy AcceptWithHoneypotOnly")`;
   row 6 `warn!(error_codes = ?…, "challenge failed")`; row 7
   `error!(error = %e, "challenge verifier unavailable")`.  Never log the
   token.  `NotConfigured` and `ChallengeUnavailable` go out as
   `ServerError`; `ChallengeRequired` and `ChallengeFailed` as `Args`.
6. **`lib.rs`.**  `pub mod challenge;` under `ssr`; re-export the public
   types.

## Required tests

`challenge/tests.rs`: one test per row of the decision table (seven),
plus: empty-string token is absent; score exactly `min_score` passes;
`expected_action` mismatch fails; `expected_action` unset ignores action.
Use a `MockVerifier(Result<ChallengeOutcome, ChallengeError>)`.
`server/tests.rs`: the three new codes carry the prefix and the correct
variant class (write a small table test on a helper if the mapping is
factored out; otherwise assert on the strings).

## Required documentation updates

`reference/api.md` new `challenge` section and the three arguments on
`submit_contact`.  Other pages in handoff 03.

## Acceptance criteria

Tests pass; gates green; a POST carrying `cf-turnstile-response=x` to a
server **without** the context is rejected with `not_configured` (curl on
the example, transcript pasted).

## Prohibited shortcuts

Verifying before validation; treating an empty token as present; a
`challenge` feature flag; logging tokens.

## Compatibility and security constraints

Additive.  Fail-closed rows are mandatory.

## Known risks

`#[server(rename = "cf-turnstile-response", default)]`: confirm the
generated struct deserialises hyphenated form keys under the `PostUrl`
encoding with a unit test that decodes
`cf-turnstile-response=abc&name=…` through the generated `SubmitContact`
type if it is reachable; if it is not reachable in tests, the curl
transcript above is the evidence.

## Required evidence

Gate outputs; test results; the curl transcript.
