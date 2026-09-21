# Testing and Local Development

## Toolchain

Rust 1.88 or later (edition 2024).  CI runs on 1.91; `clippy` lint sets can
differ between versions, so run the gates on a recent stable before
pushing.  A dedicated `msrv` job builds (never lints or tests) on 1.88
itself, so a dependency that quietly raises the MSRV is caught.

## The gates

These four commands are what CI runs.  All must pass on `main` at every
tag.

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```

`cargo test --all-features` runs the unit tests and the rustdoc examples.
Examples marked `ignore` or `no_run` are excluded on purpose (they need a
running server or environment variables).

## Test organisation

Tests live next to the module they cover, in `src/<module>/tests.rs`,
never inline.  Groups:

| Module | Covers |
|--------|--------|
| `model/tests.rs` | validation rules, trimming, honeypot detection, subject fallback |
| `config/tests.rs` | defaults |
| `error/tests.rs` | `ContactFieldErrors` JSON round-trip and sentinel parsing |
| `security/tests.rs` | header sanitisation |
| `server/tests.rs` | error-message shape (sentinel present or absent) |
| `challenge/tests.rs` | the challenge decision table, one test per row, with a mock verifier |
| `challenge/http/tests.rs` | `HttpChallengeVerifier` against a local responder: response shapes, the request (with and without `remoteip`), timeout and error mapping; live vendor tests (ignored) |
| `delivery/noop/tests.rs` | async no-op call |
| `delivery/smtp/tests.rs` | message headers, `Reply-To` encoding, body content |
| `axum_helpers/tests.rs` | closure is `Clone` |
| `tests/server/` | the crate over HTTP, in process: every behaviour a response or a delivery shows (see below) |
| `tests/browser/` | the component in headless Chrome: focus, token acquisition and refresh, explicit widget rendering (see below) |
| `tests/worker/` | the wasm32 server build (Cloudflare Workers), run in headless Chrome: extension futures that are not `Send`, the form token's JavaScript clock, `DeliveryTimeout`'s JavaScript timer, `HttpChallengeVerifier` over a stubbed `fetch`.  Run by the `browser` job's `worker tests` step |

Tests are written from the [Requirements](./requirements.md) and
[External Design](./external-design.md), not from the code: when a test
and the specification disagree, fix one of them explicitly.

## Server integration suite

`crates/leptos-hl-contact/tests/server/` tests the crate the way an
integrator's server runs it.  Each test builds a router exactly as the
documentation says to — one context closure passed to
`leptos_routes_with_context`, a small app containing `ContactForm` at
`/contact`, and no hand-written server-function route — and sends requests
through `tower::ServiceExt::oneshot`, in process.  No socket, no port, no
network.

```bash
cargo test --all-features --test server
```

The suite is compiled only with `ssr`, `axum-helpers`, `form-token` and
`delivery-timeout`, which
`--all-features` enables; `cargo test --all-features` runs it with everything
else, and so does CI.

**The harness** (`tests/server/support/`):

- **`Harness::new(Setup { … })`** builds the router with just the context
  values a test needs: delivery, the form token (plain, bound to a cookie, or
  absent), a success page, a server policy, a challenge, a filter.
  Everything is per test except the log subscriber, which is installed once
  for the binary and collects per test thread (see `support/logs.rs`).
- **`submit_nojs`** posts as a browser without JavaScript does
  (`Accept: text/html`, a `Referer`), so errors come back as a `302`.
  **`follow`** takes that redirect and renders the page it lands on.
  **`submit_fetch`** posts as `ActionForm` does with JavaScript.
- **Test doubles:** `RecordingDelivery` counts deliveries and keeps the last
  input, so "delivered or not" is asserted directly.  `FailingDelivery`
  refuses every message with a transport error and counts its calls
  (`Setup { failing_delivery: true, .. }`).  `NeverDelivery` never finishes;
  wrap it in `DeliveryTimeout` and pass it as `Setup::delivery_context`, with
  a paused clock (`#[tokio::test(start_paused = true)]`), to reach a timeout
  without waiting.  `ScriptedVerifier` answers a
  challenge with a set result and records the tokens it saw.  `FixedFilter`
  returns a set decision and counts its calls.
- **`capture_logs`** records every log event and span field for the test.

**Rules for a new case:**

- Assert what a sender or visitor can observe: status, headers, body,
  rendered page, deliveries.  Use the log capture only for properties about
  logs themselves.
- Where a behaviour exists without JavaScript and with it, test both in one
  test: the 0.4.0 honeypot regression differed only in headers.
- Never sleep.  Use `min_age_secs: 0` unless the case is about age, a large
  minimum age for "too young", and `signed_token(age_secs, nonce)` for a
  past-dated token.
- Cite the requirement or threat ID the case guards in its doc comment.

If the suite fails to compile with "found an item that was configured out",
the build artifacts came from a narrower feature set: run
`cargo clean -p leptos-hl-contact`, as described above.

## Browser tests

`crates/leptos-hl-contact/tests/browser/` tests what only happens in a
browser: focus after an error, the form token's acquisition and refresh, and
explicit rendering of a challenge widget.  Each test mounts `ContactForm` with
`leptos::mount::mount_to` into a fresh element of the test page, in headless
Chrome, and removes it afterwards.

```bash
cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock> --locked
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=$(which chromedriver) \
cargo test -p leptos-hl-contact --target wasm32-unknown-unknown --features hydrate --test browser
```

The runner refuses to run a test built with any other `wasm-bindgen`, so
install the CLI at exactly the version `Cargo.lock` resolves.  The CI
`browser` job reads that version from the lock.  `wasm-bindgen-test` is
pinned in `Cargo.toml` to the release that requires that same
`wasm-bindgen`: a caret requirement lets Cargo choose a newer one and move
`wasm-bindgen` for the whole workspace.  When the lock's `wasm-bindgen`
moves, move the pin with it.  Chrome and chromedriver major versions must
match.

Native `cargo test` does not build this suite.  It is compiled only for
`wasm32` with `hydrate` and without `ssr`.

**The harness** (`tests/browser/support/`):

- **`FetchStub`** replaces `window.fetch`.  It records every request URL and
  answers with a scripted status and body.  Success bodies are JSON (the
  server-function default), errors are `Variant|message` with a 4xx or 5xx
  status.  Nothing leaves the page.
- **`Clock`** replaces `window.setTimeout`, `clearTimeout` and `Date.now`.  A
  scheduled callback runs only when the test fires it, so "an hour later"
  takes no time at all.
- **`settle`** lets the page finish what it has started: pending promises and
  one browser task, which is when a `Response` body becomes readable.  It
  uses a `MessageChannel` message, not a timer.
- **Vendor globals** such as `window.turnstile` are plain objects with a
  recording `render`.  No vendor script is ever loaded.

**Cases:**

| File | Test | Shows |
|------|------|-------|
| `token.rs` | `without_refresh_the_token_endpoint_is_never_called` | `token_refresh_secs = None`: no request, no timer |
| `token.rs` | `an_empty_token_field_acquires_exactly_once` | an empty field fetches one token |
| `token.rs` | `an_overdue_mounted_token_refreshes_once` | an overdue rendered token refreshes on a zero-delay timer, once |
| `token.rs` | `a_fetched_token_schedules_its_refresh_from_arrival` | the next refresh is a full interval after arrival, whatever the browser clock says |
| `token.rs` | `the_hidden_token_survives_a_failed_submission` | a failed submission keeps the token and the typed input |
| `focus.rs` | `focus_moves_to_the_first_invalid_field` | focus, `aria-invalid` and `aria-describedby` after a field-error payload |
| `challenge.rs` | `a_widget_mounted_after_its_vendor_global_renders_once` | explicit `render`, once, into the form's own element |
| `challenge.rs` | `a_vendor_script_already_in_head_is_not_inserted_again` | no second vendor `<script>` |

**Rules for a new case:** stub before mounting, since the widget reads the
vendor global when the component is built.  Never wait on real time: fire the
recorded timer instead.  Cite the requirement or threat ID in the doc comment.

## Worker tests

`crates/leptos-hl-contact/tests/worker/` runs the wasm32 server paths — the
build Cloudflare Workers uses — in headless Chrome.  Chrome is not workerd,
but it has the same JavaScript APIs these paths use (`Date`, `setTimeout`,
`fetch`, `AbortController`), so the crate's own code on those paths is
exercised.  workerd itself is not in CI: 0.7.0 was tested on workerd by an
integrator, on a pre-release and in production, and later releases rely on
this suite and on integrators' reports (NFR-PORT-02).

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
CHROMEDRIVER=$(which chromedriver) \
cargo test -p leptos-hl-contact --target wasm32-unknown-unknown \
    --no-default-features --features ssr,form-token,challenge-http,delivery-timeout --test worker
```

The runner version rule is the browser suite's.  The CI `browser` job runs
this as its `worker tests` step; the `check` job lints the same binary with
`axum-helpers` added.  Native `cargo test` compiles the suite to nothing.

**The harness** (`tests/worker/support.rs`): `FetchStub` replaces
`globalThis.fetch`, found through `js_sys::global()` exactly as the verifier
finds it, records every `Request`, and answers with a status and body or
never.  Nothing leaves the page.

**Cases:**

| File | Test | Shows |
|------|------|-------|
| `not_send.rs` | `extension_futures_need_not_be_send` | a delivery, verifier and filter holding an `Rc` across `.await` build into their context types |
| `clock.rs` | `a_token_issued_now_verifies` | a token issued now verifies, and its timestamp is within 2 s of `Date.now()` |
| `clock.rs` | `a_token_is_too_young_before_its_minimum_age` | the minimum age uses the same clock |
| `timer.rs` | `a_delivery_that_never_finishes_times_out` | `DeliveryTimeout` returns `Timeout(limit)` after the limit (a real 50 ms wait) |
| `timer.rs` | `a_delivery_that_finishes_in_time_returns_its_result` | `Ok` and a transport error pass through |
| `fetch_verifier.rs` | `the_request_refuses_redirects_and_posts_the_form` | `POST`, `redirect: "manual"`, form content type, `secret`, `response`, `remoteip`; a finished request is not aborted |
| `fetch_verifier.rs` | `a_redirect_is_unavailable` | a 302 is `Unavailable` |
| `fetch_verifier.rs` | `a_verdict_is_parsed` | success and failure with error codes |
| `fetch_verifier.rs` | `a_silent_vendor_is_a_timeout` | `Timeout` at the limit, and the request is aborted |
| `fetch_verifier.rs` | `an_empty_secret_sends_nothing` | `Misconfigured`, no request |
| `fetch_verifier.rs` | `a_dropped_verification_aborts_its_request` | dropping the verification mid-flight aborts its request |

**Rules for a new case:** install the stub before the call and keep it alive
for the test.  A real wait is allowed only where a JavaScript timer is the
thing under test, and only for tens of milliseconds.  Cite the requirement
ID in the doc comment.

## Requirement traceability

Every **MUST** row of the [Requirements Specification](./requirements.md)
appears here once, with the tests that assert it.

- **L1** is a unit test, `module::test`, in `src/<module>/tests.rs`, or
  `module::file::test` when it lives in `src/<module>/tests/<file>.rs`.
- **L2** is `file::test` in `tests/server/`.
- **L3** is `file::test` in `tests/browser/`, or `worker::file::test` in
  `tests/worker/` (a wasm32 server build, also run in headless Chrome).
- **none** means no automated test asserts the requirement.  The last column
  then says how it is verified instead.

A test is listed only for what it asserts.  When tests cover part of a row,
the last column names the part they leave out.

MUST rows are:

- the functional rows whose Level is MUST (84);
- the non-functional rows whose wording says MUST (28).

The non-functional tables have no Level column.  Their rows without MUST
are SHOULDs, and are not listed: NFR-PERF-03, NFR-DEP-02 and NFR-DOC-04.

**Keeping it true.**  A handoff that adds a MUST requirement, or changes the
behaviour behind one, updates that row in the same commit.  So does a change
that adds, renames or removes a test named here.  Every test in
`tests/server/`, `tests/browser/` and `tests/worker/` cites at least one requirement or
threat ID in its doc comment; `grep -rn "FR-UI-12" crates/` finds a row's
tests from the code side.

### Functional requirements

| Requirement | L1 unit | L2 server | L3 browser | Otherwise, or not covered |
|---|---|---|---|---|
| FR-UI-01 | `components::form_renders_all_ids_once`, `components::subject_hidden_when_option_false`, `components::attributes::required_fields_carry_both_required_attributes`, `components::site_fields::no_site_fields_render_the_markup_of_0_7_byte_for_byte` | — | `site_fields::the_rows_render_with_their_names_ids_and_required` | the bound of four site-defined fields: FR-FIELD-01 |
| FR-UI-02 | `config::field_text_substitutes_in_a_translated_label`, `components::the_noscript_message_escapes_label_and_class` | — | `pending::the_submit_button_is_busy_while_sending` | every other string: review of `ContactFormLabels` |
| FR-UI-03 | `config::classes_default_is_all_empty`, `components::the_noscript_message_escapes_label_and_class`, `components::attributes::an_opted_out_honeypot_has_its_class_and_no_inline_style`, `components::attributes::the_honeypot_class_is_added_beside_the_inline_style` | — | — | a hook on every structural element: review |
| FR-UI-04 | — | `validation::field_errors_round_trip_without_javascript`, `validation::a_banner_error_sets_no_field_error`, `delivery::the_success_page_is_applied_when_configured` | `focus::focus_moves_to_the_first_invalid_field`, `token::the_hidden_token_survives_a_failed_submission` | the pending state and inline success: review |
| FR-UI-05 | — | — | `pending::the_submit_button_is_busy_while_sending` | — |
| FR-UI-06 | — | `delivery::the_success_page_is_applied_when_configured` | — | inline success with JavaScript: review |
| FR-UI-07 | — | — | `focus::focus_moves_to_the_first_invalid_field` | — |
| FR-UI-08 | `config::field_text_renders_required_and_line_breaks` | `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-UI-09 | `config::code_text_maps_unexpected_to_delivery_failed`, `config::code_text_renders_delivery_timeout` | `delivery::a_delivery_error_reaches_the_client_only_as_delivery_failed`, `routing::a_missing_delivery_context_is_not_configured`, `validation::a_banner_error_sets_no_field_error`, `delivery::a_delivery_timeout_reaches_the_client_as_delivery_timeout` | — | — |
| FR-UI-10 | `components::attributes::the_honeypot_is_hidden_from_everyone`, `components::attributes::an_opted_out_honeypot_has_its_class_and_no_inline_style`, `config::the_honeypot_defaults_keep_the_inline_style` | — | — | with `honeypot_inline_style: false` the site's CSS hides the wrapper: documentation (Styling) |
| FR-UI-11 | `config::options_default_shows_subject`, `config::options_effective_len_is_clamped`, `components::subject_hidden_when_option_false`, `components::attributes::required_fields_carry_both_required_attributes`, `components::attributes::maxlength_matches_the_validator` | — | — | — |
| FR-UI-12 | `components::hidden_token_is_rendered_from_context`, `components::the_reactive_token_attribute_renders_the_ssr_value`, `server::a_mounted_token_refreshes_at_its_refresh_point`, `server::an_overdue_mounted_token_refreshes_immediately`, `server::a_fast_browser_clock_costs_one_immediate_refresh`, `server::a_slow_browser_clock_is_capped_at_the_interval`, `config::options_default_never_calls_the_token_endpoint` | `binding::binding_reuses_the_browser_nonce_across_renders`, `binding::the_token_endpoint_reuses_the_nonce` | `token::without_refresh_the_token_endpoint_is_never_called`, `token::an_empty_token_field_acquires_exactly_once`, `token::an_overdue_mounted_token_refreshes_once`, `token::a_fetched_token_schedules_its_refresh_from_arrival`, `token::the_hidden_token_survives_a_failed_submission` | — |
| FR-SUB-01 | — | `routing::context_in_the_one_closure_reaches_submit_contact`, `delivery::a_valid_submission_is_delivered_once_in_both_forms` | — | — |
| FR-SUB-02 | `form_token::age_is_checked_before_the_signature` | **none** | — | the order of the steps in `submit_contact`: review of `server.rs` |
| FR-SUB-03 | `model::whitespace_only_name_yields_required_code`, `model::empty_subject_uses_fallback` | `validation::a_blank_subject_is_delivered_as_absent` | — | — |
| FR-SUB-04 | `model::honeypot_input_is_detected` | `silent::a_silent_outcome_is_indistinguishable_from_delivery`, `logging::no_personal_data_or_secret_is_logged` | — | the `warn` level: review |
| FR-SUB-05 | `model::valid_input_passes_validation` and the other `model::` rule tests | `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-SUB-06 | `error::field_errors_roundtrip_json`, `server::field_error_message_has_prefix` | `validation::each_rule_rejects_with_its_field_code`, `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-SUB-07 | `config::policy_check_requires_subject_when_set`, `config::policy_check_counts_characters_not_bytes`, `config::policy_check_reports_both_errors_at_once` | `policy::server_policy_requires_the_subject`, `policy::server_policy_counts_the_message_in_characters` | — | — |
| FR-SUB-08 | — | `routing::a_missing_delivery_context_is_not_configured`, `logging::no_personal_data_or_secret_is_logged` | — | — |
| FR-SUB-09 | — | `delivery::a_delivery_error_reaches_the_client_only_as_delivery_failed`, `logging::no_personal_data_or_secret_is_logged`, `delivery::a_delivery_timeout_reaches_the_client_as_delivery_timeout` | — | — |
| FR-SUB-10 | **none** | **none** | — | review: `std::env` appears in `src/` only inside rustdoc examples |
| FR-VAL-01 | `model::empty_name_fails`, `model::newline_in_name_fails`, `model::over_long_name_still_yields_length_code`, `model::newline_in_name_yields_line_breaks_code` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-VAL-02 | `model::invalid_email_fails`, `model::bad_email_yields_format_code`, `model::email::reply_to_addresses_are_accepted`, `model::email::a_single_label_domain_is_a_format_error`, `model::email::an_address_literal_is_a_format_error`, `model::email::an_empty_domain_label_is_a_format_error`, `model::email::a_254_character_address_is_accepted`, `model::email::a_255_character_address_is_a_length_error`, `model::email::a_long_invalid_address_reports_its_length`, `model::email::a_blank_email_keeps_its_format_code` | `validation::each_rule_rejects_with_its_field_code`, `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-VAL-03 | `model::newline_in_subject_fails`, `model::over_long_subject_yields_length_code_with_zero_min`, `model::empty_subject_uses_fallback` | `validation::each_rule_rejects_with_its_field_code`, `validation::a_blank_subject_is_delivered_as_absent` | — | — |
| FR-VAL-04 | `model::too_long_message_fails`, `model::empty_message_yields_required_code`, `model::over_long_message_yields_length_code_at_the_ceiling` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-VAL-05 | `model::honeypot_input_is_detected` | `silent::a_silent_outcome_is_indistinguishable_from_delivery` | — | — |
| FR-VAL-06 | `form_token::issued_token_verifies_once_old_enough`, `form_token::malformed_tokens_are_rejected`, `form_token::an_old_token_is_expired`, `form_token::a_token_younger_than_the_minimum_is_too_young` | `form_token::a_missing_malformed_or_expired_token_is_rejected`, `form_token::a_too_young_token_is_retryable`, `form_token::a_0_4_csrf_token_field_is_no_longer_accepted` | `worker::clock::a_token_issued_now_verifies`, `worker::clock::a_token_is_too_young_before_its_minimum_age` (wasm32 server, headless Chrome) | — |
| FR-VAL-07 | `model::message_length_counts_characters`, `config::policy_check_counts_characters_not_bytes`, `components::attributes::maxlength_matches_the_validator`, `model::email::a_255_character_address_is_a_length_error` | `policy::server_policy_counts_the_message_in_characters` | — | — |
| FR-VAL-08 | `model::message_ceiling_constant_is_enforced_by_validator`, `config::options_effective_len_is_clamped`, `config::policy_check_clamps_to_ceiling`, `config::policy_default_matches_ceiling`, `components::attributes::maxlength_matches_the_validator` | — | — | — |
| FR-ABUSE-01 | `model::honeypot_input_is_detected` | `silent::a_silent_outcome_is_indistinguishable_from_delivery` (no honeypot configuration in the harness) | — | — |
| FR-ABUSE-02 | `form_token::binding_cookie_requires_a_matching_nonce`, `axum_helpers::the_prefix_is_applied_at_the_defaults`, `axum_helpers::a_bare_name_is_ignored_while_the_prefix_is_in_force`, `axum_helpers::two_renders_for_one_browser_share_a_nonce` | `binding::binding_uses_the_host_prefix_at_the_defaults`, `binding::binding_requires_the_matching_cookie`, `binding::binding_rejects_a_tossed_bare_cookie`, `binding::binding_reuses_the_browser_nonce_across_renders`, `binding::the_token_endpoint_reuses_the_nonce` | — | the statement that Origin validation stays the application's control: documentation (`security/form-token.md`) |
| FR-ABUSE-03 | `form_token::an_old_token_is_expired`, `form_token::a_far_future_token_is_rejected`, `form_token::a_tampered_signature_is_a_bad_signature` | `form_token::a_missing_malformed_or_expired_token_is_rejected` | — | constant-time comparison: review (`form_token.rs`, `constant_time_eq`); see NFR-SEC-03 |
| FR-ABUSE-04 | `form_token::a_fetched_token_is_refused_without_the_config` | `form_token::a_missing_token_config_fails_closed` | — | the `error` log: review |
| FR-ABUSE-05 | **none** | **none** | — | documentation, and the `axum-with-security` example compiled by the CI `examples` job |
| FR-ABUSE-06 | **none** | **none** | — | documentation, and the `axum-with-security` example compiled by the CI `examples` job |
| FR-ABUSE-07 | **none** | **none** | — | documentation, and the examples compiled by the CI `examples` job |
| FR-ABUSE-08 | **none** | **none** | — | documentation |
| FR-ABUSE-09 | `server::succeed_applies_the_success_redirect_only_when_configured` | `silent::a_silent_outcome_is_indistinguishable_from_delivery` | — | — |
| FR-ABUSE-10 | `challenge::row_5_passed_proceeds`, `challenge::row_6_not_passed_fails`, `challenge::http::each_provider_uses_its_vendor_endpoint_by_default`, `components::turnstile_renders_its_element_and_script`, `components::hcaptcha_renders_its_element_and_script_and_omits_auto_theme`, `components::recaptcha_v2_renders_its_element_and_script_and_omits_auto_theme`, `components::recaptcha_v3_renders_the_hidden_input_render_url_and_submit_script`, `challenge::http::the_form_body_carries_remoteip_when_provided`, `challenge::http::verify_is_verify_request_without_an_ip`, `config::widget_script_nonce_follows_the_csp3_grammar`, `components::a_url_safe_nonce_is_on_every_script_tag`, `config::challenge_size_maps_to_the_vendor_value_per_provider`, `components::turnstile_size_is_a_data_attribute_when_set`, `components::default_size_leaves_the_turnstile_element_unchanged_from_0_7` | `challenge::challenge_decision_table_rows_1_to_7`, `challenge::the_client_ip_reaches_the_verifier` | `challenge::a_widget_mounted_after_its_vendor_global_renders_once`, `challenge::a_vendor_script_already_in_head_is_not_inserted_again`, `challenge::an_explicit_render_passes_the_compact_size`, `challenge::an_explicit_render_omits_size_by_default`, `worker::fetch_verifier::the_request_refuses_redirects_and_posts_the_form`, `worker::fetch_verifier::a_verdict_is_parsed` (wasm32 server, headless Chrome) | the live vendor contracts: the `#[ignore]`d `live_` tests, by hand; the client-side script insertion (`append_script`) has no browser test, because a negative control would fetch the real vendor script |
| FR-ABUSE-11 | `challenge::row_3_context_no_token_under_reject_is_required`, `challenge::row_4_context_no_token_under_accept_proceeds`, `components::reject_renders_noscript_and_accept_does_not`, `config::no_js_policy_defaults_to_reject` | `challenge::challenge_decision_table_rows_1_to_7` | — | — |
| FR-ABUSE-12 | `challenge::row_7_verifier_error_is_unavailable_for_every_variant`, `challenge::http::a_server_error_is_unavailable`, `challenge::http::a_silent_server_is_a_timeout`, `challenge::http::a_redirect_is_not_followed_and_is_unavailable`, `challenge::http::an_empty_secret_is_misconfigured_and_sends_nothing`, `challenge::http::the_default_timeout_is_five_seconds` | `challenge::challenge_decision_table_rows_1_to_7`, `challenge::a_widget_with_an_empty_secret_refuses_with_and_without_a_token` | `worker::fetch_verifier::the_request_refuses_redirects_and_posts_the_form`, `worker::fetch_verifier::a_redirect_is_unavailable`, `worker::fetch_verifier::a_silent_vendor_is_a_timeout`, `worker::fetch_verifier::an_empty_secret_sends_nothing`, `worker::fetch_verifier::a_dropped_verification_aborts_its_request` (wasm32 server, headless Chrome) | — |
| FR-ABUSE-15 | `challenge::http::the_form_body_carries_remoteip_when_provided`, `challenge::a_challenge_request_debug_redacts_the_token_and_ip`, `challenge::a_client_ip_debug_redacts_the_address` | `challenge::the_client_ip_reaches_the_verifier`, `logging::no_personal_data_or_secret_is_logged` | `worker::fetch_verifier::the_request_refuses_redirects_and_posts_the_form` (wasm32 server, headless Chrome) | that the crate never reads a header for the IP: review (`server.rs` reads it only from `ChallengeClientIp`) |
| FR-DEL-01 | `axum_helpers::delivery_context_fn_is_clone` | `delivery::a_valid_submission_is_delivered_once_in_both_forms` (a test double provided as `Arc<dyn ContactDelivery>`) | — | object safety and `Send`: checked at compile time |
| FR-DEL-02 | — | `validation::a_blank_subject_is_delivered_as_absent`, `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-DEL-03 | `delivery::noop::noop_delivery_succeeds` | — | — | the `debug` log without PII: review |
| FR-DEL-04 | **none** | **none** | — | review of `SmtpTlsMode` in `delivery/smtp.rs`; a TLS session needs a relay, so not testable offline |
| FR-DEL-05 | `delivery::smtp::from_uses_configured_address`, `delivery::smtp::reply_to_uses_user_email`, `delivery::smtp::reply_to_with_special_chars_in_name`, `delivery::smtp::reply_to_uses_mailbox_new_not_string_parse`, `delivery::smtp::message_builder_creates_expected_headers` | — | — | — |
| FR-DEL-06 | `delivery::smtp::body_includes_expected_fields` | — | — | — |
| FR-DEL-07 | — | `delivery::a_valid_submission_is_delivered_once_in_both_forms`, `delivery::a_delivery_error_reaches_the_client_only_as_delivery_failed` (custom backends in `tests/server/support/doubles.rs`) | — | the error-text contract for implementers: documentation (`ContactDelivery`'s `# Errors`, the delivery guide, External Design §4.4.1) |
| FR-CFG-01 | **none** | **none** | — | CI: `clippy` with all features and with `ssr,smtp-lettre,axum-helpers`, and the `browser` job with `hydrate` alone |
| FR-CFG-02 | `axum_helpers::delivery_context_fn_is_clone` | `routing::context_in_the_one_closure_reaches_submit_contact` | — | the documentation of each context value: documentation |
| FR-CFG-03 | `challenge::http::an_empty_secret_is_misconfigured_and_sends_nothing` | `routing::a_missing_delivery_context_is_not_configured`, `form_token::a_missing_token_config_fails_closed`, `challenge::challenge_decision_table_rows_1_to_7` (row 2) | — | the examples' startup panics: review |
| FR-CFG-04 | `form_token::debug_redacts_the_secret`, `challenge::http::debug_redacts_the_secret`, `config::redirect_debug_does_not_expose_the_executor`, `delivery::smtp::debug_redacts_the_password` | — | — | — |
| FR-CFG-05 | `form_token::default_config_has_a_two_second_minimum_age`, `axum_helpers::cookie_defaults_are_the_documented_ones`, `config::policy_default_matches_ceiling`, `config::no_js_policy_defaults_to_reject`, `challenge::the_policy_defaults_are_the_documented_ones`, `challenge::http::the_default_timeout_is_five_seconds` | — | — | the STARTTLS default and the one-hour token TTL: review |
| FR-I18N-01 | `config::field_text_substitutes_in_a_translated_label`, `components::the_noscript_message_escapes_label_and_class` | — | — | — |
| FR-I18N-02 | `error::contact_error_code_round_trips_through_the_wire_string`, `config::code_text_maps_unexpected_to_delivery_failed`, `config::field_text_substitutes_min_and_max`, `error::delivery_timeout_round_trips_through_the_wire_string`, `config::code_text_renders_delivery_timeout` | `validation::field_errors_round_trip_without_javascript`, `delivery::a_delivery_timeout_reaches_the_client_as_delivery_timeout` | — | — |
| FR-I18N-04 | **none** | **none** | — | review: the rendered markup carries no `dir`, `lang` or locale formatting |
| FR-I18N-05 | `model::message_length_counts_characters`, `delivery::smtp::reply_to_with_special_chars_in_name` | `validation::each_rule_rejects_with_its_field_code` (a non-ASCII message) | — | a non-ASCII value reaching delivery unchanged, and header encoding: review (lettre) |
| FR-A11Y-01 | `components::attributes::every_input_has_a_label_for_it` | — | — | — |
| FR-A11Y-02 | `components::attributes::required_fields_carry_both_required_attributes` | — | — | — |
| FR-A11Y-03 | — | `validation::field_errors_round_trip_without_javascript` | `focus::focus_moves_to_the_first_invalid_field` | — |
| FR-A11Y-04 | — | `validation::a_banner_error_sets_no_field_error` (the assertive banner) | — | the polite success region and field alerts: review |
| FR-A11Y-05 | — | — | `pending::the_submit_button_is_busy_while_sending` | — |
| FR-A11Y-06 | `components::attributes::the_honeypot_is_hidden_from_everyone`, `components::attributes::an_opted_out_honeypot_has_its_class_and_no_inline_style` | — | — | — |
| FR-A11Y-07 | **none** | **none** | **none** | review: native controls only, and the crate ships no CSS |
| FR-A11Y-08 | **none** | **none** | **none** | review: every state has text; the crate ships no colours |
| FR-A11Y-09 | **none** | **none** | **none** | by construction and review; no automated accessibility audit runs |
| FR-PE-01 | — | `delivery::a_valid_submission_is_delivered_once_in_both_forms` | — | — |
| FR-PE-02 | — | `validation::field_errors_round_trip_without_javascript`, `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-PE-03 | — | `delivery::the_success_page_is_applied_when_configured` | — | — |
| FR-PE-04 | — | `validation::each_rule_rejects_with_its_field_code`, `policy::server_policy_requires_the_subject`, `policy::server_policy_counts_the_message_in_characters`, `delivery::a_valid_submission_is_delivered_once_in_both_forms` | — | — |
| FR-FIELD-01 | `config::site_fields_accept_the_documented_example`, `config::no_site_fields_is_the_default_and_valid`, `config::site_fields_are_at_most_four`, `config::site_field_keys_follow_the_charset_and_length`, `config::every_reserved_key_is_refused`, `config::a_duplicate_key_is_refused`, `config::a_label_is_not_empty_after_trimming`, `config::a_line_max_len_is_one_to_two_hundred`, `config::a_text_max_len_is_one_to_the_message_ceiling`, `config::a_choice_lists_two_to_twenty_options`, `config::choice_keys_follow_the_charset_and_are_unique`, `config::a_choice_label_is_not_empty`, `components::site_fields::the_rows_sit_between_the_subject_and_the_message_in_definition_order` | `site_fields::an_unknown_key_or_too_many_keys_reject_the_whole_submission` (a fifth key) | — | the three kinds only: `SiteFieldKind` is a closed enum, so no other kind can be written; no logic and no layout beyond the existing classes: review |
| FR-FIELD-02 | — | `site_fields::the_page_renders_the_shared_definition`, `site_fields::a_server_field_the_form_did_not_render_shows_the_generic_message` | `site_fields::an_error_for_an_unrendered_key_shows_the_generic_message` | passing one definition to both the prop and the policy: documentation (Customization); a mismatch fails closed: FR-FIELD-03 |
| FR-FIELD-03 | `model::site_fields::five_keys_are_refused`, `model::site_fields::an_unknown_key_is_refused`, `model::site_fields::a_key_with_a_newline_is_refused_not_a_crash`, `model::site_fields::a_definition_with_no_fields_accepts_no_map_and_refuses_any_key` | `site_fields::an_unknown_key_or_too_many_keys_reject_the_whole_submission`, `site_fields::a_site_without_site_fields_refuses_any_field_key`, `site_fields::a_honeypot_hit_is_silent_whatever_the_site_fields_hold`, `site_fields::a_malformed_map_shows_the_generic_message`, `site_fields::too_many_keys_log_the_count_and_never_a_key`, `site_fields::an_unknown_key_logs_its_count_and_never_the_key`, `site_fields::several_unknown_keys_are_counted` | — | a key sent twice or nested fails in the decoder before the crate runs: the L2 test above shows the visitor's outcome, and the crate cannot log it |
| FR-FIELD-04 | `model::site_fields::an_unlisted_choice_is_format`, `model::site_fields::a_listed_choice_carries_its_key_and_its_label` | `site_fields::each_rule_reports_its_code_under_the_key` (an unlisted choice) | — | — |
| FR-FIELD-05 | `model::site_fields::a_required_field_that_is_blank_or_missing_is_required`, `model::site_fields::a_value_is_trimmed`, `model::site_fields::a_line_at_its_limit_passes_and_one_over_is_length`, `model::site_fields::length_counts_characters_not_bytes`, `model::site_fields::a_line_with_a_line_break_is_line_breaks`, `model::site_fields::length_is_reported_before_line_breaks`, `model::site_fields::a_text_field_takes_line_breaks_and_has_a_length`, `model::site_fields::several_errors_are_reported_together`, `error::json_without_site_field_errors_is_byte_identical_to_0_7`, `error::a_0_7_payload_still_parses`, `error::site_field_errors_round_trip_through_json`, `error::a_site_field_error_alone_is_not_empty` | `site_fields::each_rule_reports_its_code_under_the_key`, `site_fields::the_error_text_is_the_existing_label_text`, `site_fields::site_field_errors_and_built_in_errors_arrive_together` | `site_fields::a_field_error_shows_under_its_field` | — |
| FR-FIELD-06 | `model::site_fields::valid_values_follow_definition_order`, `model::site_fields::labels_come_from_the_definition`, `model::site_fields::an_absent_optional_field_is_omitted`, `delivery::smtp::a_body_without_site_fields_is_byte_identical_to_0_7`, `delivery::smtp::site_fields_have_one_block_each_between_the_subject_and_the_message`, `delivery::smtp::a_multi_line_text_value_is_kept_intact`, `delivery::smtp::a_site_value_never_reaches_a_header` | `site_fields::a_valid_submission_is_delivered_with_the_values_in_definition_order`, `site_fields::a_blank_optional_field_is_left_out_of_delivery`, `site_fields::no_fields_at_all_with_an_empty_definition_is_todays_behaviour` | — | a custom backend's handling of the values: documentation (Delivery Backends) |
| FR-FIELD-07 | `config::an_error_message_never_carries_a_label` | `site_fields::site_field_values_and_request_keys_are_never_logged`, `site_fields::too_many_keys_log_the_count_and_never_a_key`, `site_fields::an_unknown_key_logs_its_count_and_never_the_key`, `site_fields::several_unknown_keys_are_counted` | — | `Debug` on `ContactInput`, which shows the values as it shows `message`: unchanged, out of scope (RFC 015 D6) |
| FR-FIELD-08 | `components::site_fields::each_kind_renders_its_control_with_its_name_and_id`, `components::site_fields::a_choice_starts_with_the_empty_option_then_its_choices_in_order`, `components::site_fields::required_fields_carry_required_and_aria_required`, `components::site_fields::a_field_without_an_error_is_not_marked_invalid`, `components::site_fields::the_rows_use_the_existing_classes`, `components::site_fields::the_rows_are_still_before_the_message_without_a_subject`, `components::site_fields::labels_are_escaped` | `site_fields::each_rule_reports_its_code_under_the_key` (the no-JavaScript page: `aria-invalid`, `aria-describedby`, the error paragraph) | `site_fields::the_rows_render_with_their_names_ids_and_required`, `site_fields::the_submitted_body_carries_the_site_fields`, `site_fields::a_field_error_shows_under_its_field`, `site_fields::focus_goes_to_the_first_invalid_site_field_before_the_message`, `site_fields::the_built_in_fields_keep_their_place_around_the_site_fields`, `site_fields::a_failed_submission_keeps_what_was_typed_and_selected` | without JavaScript nothing is preserved, for any field, as before (RFC 015 A6) |
| FR-OBS-01 | — | `logging::no_personal_data_or_secret_is_logged` | — | each event's level: review |
| FR-OBS-02 | `server::pii_not_present_in_expected_log_messages` | `logging::no_personal_data_or_secret_is_logged`, `site_fields::site_field_values_and_request_keys_are_never_logged` | — | — |
| FR-OBS-03 | — | `logging::no_personal_data_or_secret_is_logged` | — | — |
| FR-OBS-04 | **none** | **none** | — | review: no storage in the crate or its dependencies |

### Non-functional requirements

| Requirement | L1 unit | L2 server | L3 browser | Otherwise, or not covered |
|---|---|---|---|---|
| NFR-SEC-01 | **none** | **none** | **none** | the `browser` CI job builds the crate without `ssr`, where the configuration types do not exist; review |
| NFR-SEC-02 | `security::header_injection_attempt_is_sanitised`, `model::newline_in_name_fails`, `model::newline_in_subject_fails` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| NFR-SEC-03 | `form_token::a_signature_one_hex_digit_off_is_a_bad_signature`, `form_token::a_signature_one_byte_short_is_a_bad_signature`, `form_token::a_non_hex_signature_is_a_bad_signature`, `form_token::a_same_length_wrong_binding_is_a_binding_mismatch` (each comparison refuses a near miss) | **none** | — | that the comparisons are constant-time: review of `verify_slice` (`hmac`) and `ConstantTimeEq::ct_eq` (`subtle`) in `verify_form_token`; no test measures timing |
| NFR-SEC-04 | `challenge::row_7_verifier_error_is_unavailable_for_every_variant`, `challenge::http::a_server_error_is_unavailable` | `form_token::a_missing_token_config_fails_closed`, `routing::a_missing_delivery_context_is_not_configured`, `challenge::challenge_decision_table_rows_1_to_7` | — | — |
| NFR-SEC-05 | **none** | **none** | — | review, at each RFC that adds a data flow |
| NFR-SEC-06 | **none** | **none** | — | review of the dependency tree at the release security audit |
| NFR-PRIV-01 | `server::pii_not_present_in_expected_log_messages` | `logging::no_personal_data_or_secret_is_logged` | — | never persisted: see FR-OBS-04 |
| NFR-PRIV-02 | **none** | **none** | — | documentation (per-provider notes) |
| NFR-COMPAT-01 | **none** | **none** | **none** | CI builds: `cargo test --all-features` (the `ssr`, `hydrate` and `islands` features together) and the `browser` job (`hydrate` on wasm32); Islands mode behaviour: review |
| NFR-COMPAT-02 | **none** | **none** | **none** | CI: the `msrv` job builds the crate on Rust 1.88 (native, wasm32 `hydrate`, and the Workers feature set), each `--locked` |
| NFR-COMPAT-03 | **none** | **none** | — | review of `Cargo.toml` (`axum` is optional, behind `axum-helpers`); the `browser` job builds the crate without it |
| NFR-COMPAT-04 | `config::settings_serialized_by_0_6_still_deserialize` | `form_token::a_0_4_csrf_token_field_is_no_longer_accepted` | — | the rest of the policy: review at release |
| NFR-COMPAT-05 | **none** | **none** | **none** | review at release; the SSR tests in `components::` pin the ids, field names and the ARIA attributes tested above, so a change to those fails a test |
| NFR-PORT-01 | **none** | **none** | **none** | CI: the `check` job builds `default = []` for wasm32 (`cargo check -p leptos-hl-contact --target wasm32-unknown-unknown`); the `browser` job and the example's wasm check build it with `hydrate` |
| NFR-PORT-02 | **none** | **none** | `worker::not_send::extension_futures_need_not_be_send`, `worker::clock::a_token_issued_now_verifies`, `worker::clock::a_token_is_too_young_before_its_minimum_age`, `worker::timer::a_delivery_that_never_finishes_times_out`, `worker::timer::a_delivery_that_finishes_in_time_returns_its_result`, `worker::fetch_verifier::the_request_refuses_redirects_and_posts_the_form`, `worker::fetch_verifier::a_redirect_is_unavailable`, `worker::fetch_verifier::a_verdict_is_parsed`, `worker::fetch_verifier::a_silent_vendor_is_a_timeout`, `worker::fetch_verifier::an_empty_secret_sends_nothing`, `worker::fetch_verifier::a_dropped_verification_aborts_its_request` (wasm32 server, headless Chrome) | CI: the `check` job's Workers step builds and lints `ssr,form-token,challenge-http,axum-helpers,delivery-timeout` for wasm32; runtime on workerd (`wrangler dev`, a live Worker): the reflerd.com team's report before 0.7.0 |
| NFR-PERF-01 | **none** | **none** | — | review |
| NFR-PERF-02 | **none** | **none** | — | review; the input limits themselves: FR-VAL-01 to -04 |
| NFR-DEP-01 | **none** | **none** | — | review of `Cargo.toml`; CI builds with and without the optional features |
| NFR-DOC-01 | **none** | **none** | — | the documentation verification pass at each release |
| NFR-DOC-02 | the doctests run by `cargo test --all-features` | — | — | — |
| NFR-DOC-03 | **none** | **none** | — | the documentation verification pass; `mdbook test` does not run in CI |
| NFR-TEST-01 | **none** | **none** | — | the CI workflow itself |
| NFR-TEST-02 | — | — | — | this table |
| NFR-TEST-03 | — | the `tests/server/` suite; each case is listed in its row above | — | — |
| NFR-TEST-04 | **none** | **none** | **none** | review |
| NFR-REL-01 | **none** | **none** | **none** | review at release |
| NFR-REL-02 | **none** | **none** | — | review at release |
| NFR-REL-03 | **none** | **none** | — | the release process |

## Live challenge tests

The `challenge-http` verifiers have tests that call the real vendor
endpoints with the vendors' published test keys.  They are `#[ignore]`d, so
neither `cargo test` nor CI runs them.  Run them by hand before a release
that touches `challenge/http.rs`:

```bash
cargo test -p leptos-hl-contact --no-default-features --features challenge-http --lib -- --ignored live_ --nocapture
```

They need outbound HTTPS to `challenges.cloudflare.com`, `api.hcaptcha.com`
and `www.google.com`.  Google's test secret accepts any token, so the
reCAPTCHA test shows the round trip, not a real check.

## Running the examples

`axum-basic` is server-only and needs nothing but Cargo:

```bash
cd examples/axum-basic && cargo run
```

`axum-with-security` ships a WASM client, so it is built and served with
[cargo-leptos](https://github.com/leptos-rs/cargo-leptos):

```bash
cargo install cargo-leptos
cd examples/axum-with-security
FORM_TOKEN_SECRET=$(openssl rand -hex 32) ALLOWED_ORIGIN=http://127.0.0.1:3000 cargo leptos watch
```

Use `cargo leptos serve` for a one-shot build without the file watcher.
`ALLOWED_ORIGIN` must match the scheme, host and port in the address bar
exactly, or the origin check rejects every POST with `403`.

Without cargo-leptos the server binary still compiles and runs, but no
client bundle is produced, so the form falls back to the plain-POST path:

```bash
cd examples/axum-with-security && cargo check --features ssr
```

CI checks both targets of this example — `--features ssr` and
`--features hydrate --target wasm32-unknown-unknown --lib` — but does not run
cargo-leptos.  It checks the examples with `--locked`, so a stale example lock
file fails at push time rather than at the release.

Both examples use `NoopDelivery`; switch to `LettreSmtpDelivery` and point it
at MailHog to see real messages
([Delivery Backends](../guides/delivery-backends.md#testing-delivery-locally)).

### Testing hydrated behaviour in a browser

Two rules, both learned the hard way:

**Rebuild the client bundle first, every time.** `cargo clean -p` does not
remove `target/site/pkg`, and a stale `.wasm` will happily serve code from
before your change — once making it look as though a fixed defect was still
present. Delete the outputs and rebuild, then check the timestamps:

```bash
cd examples/axum-with-security
rm -rf target/front target/site
cargo leptos build
ls -l target/site/pkg     # every file must be newer than your edit
```

**A build with narrower features can poison a later `--all-features` run.**
If `cargo test --all-features` reports a doctest failing with "found an item
that was configured out ... gated behind the `smtp-lettre` feature", the
crate's artefacts were last written by a build with a narrower feature set.
`cargo clean -p leptos-hl-contact` and re-run; CI never sees this because it
builds from scratch.  Do not read it as a defect until you have cleaned.

**Rebuild the example with `cargo leptos build`, not `cargo build`.** After
editing an example, a plain `cargo build` refreshes the binary but not the
generated bundle, and the two then disagree about the WASM filename: the
page asks for `<name>_bg.wasm`, gets a 404, and silently falls back to the
plain-POST path. The symptoms are a `TypeError: Failed to execute 'compile'
on 'WebAssembly': HTTP status code is not ok` in the console and a submit
recorded as a `Document` request where a `Fetch` was expected.

**Set `form.noValidate` before testing server-side validation.** The form
carries `required` and `type="email"`, so the browser blocks a submit with a
deliberately invalid value and the request never reaches the server — the
test then proves nothing:

```js
document.querySelector('form').noValidate = true;
```

## Building this book

```bash
cd docs && mdbook build
```
