# Testing and Local Development

## Toolchain

Rust 1.85 or later (edition 2024).  CI runs on 1.91; `clippy` lint sets can
differ between versions, so run the gates on a recent stable before
pushing.

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
| `challenge/http/tests.rs` | `HttpChallengeVerifier` against a local responder: response shapes, the request, timeout and error mapping; live vendor tests (ignored) |
| `delivery/noop/tests.rs` | async no-op call |
| `delivery/smtp/tests.rs` | message headers, `Reply-To` encoding, body content |
| `axum_helpers/tests.rs` | closure is `Clone` |
| `tests/server/` | the crate over HTTP, in process: every behaviour a response or a delivery shows (see below) |
| `tests/browser/` | the component in headless Chrome: focus, token acquisition and refresh, explicit widget rendering (see below) |

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

The suite is compiled only with `ssr`, `axum-helpers` and `form-token`, which
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
  (`Setup { failing_delivery: true, .. }`).  `ScriptedVerifier` answers a
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

## Requirement traceability

Every **MUST** row of the [Requirements Specification](./requirements.md)
appears here once, with the tests that assert it.

- **L1** is a unit test, `module::test`, in `src/<module>/tests.rs`, or
  `module::file::test` when it lives in `src/<module>/tests/<file>.rs`.
- **L2** is `file::test` in `tests/server/`.
- **L3** is `file::test` in `tests/browser/`.
- **none** means no automated test asserts the requirement.  The last column
  then says how it is verified instead.

A test is listed only for what it asserts.  When tests cover part of a row,
the last column names the part they leave out.

MUST rows are:

- the functional rows whose Level is MUST (75);
- the non-functional rows whose wording says MUST (27).

The non-functional tables have no Level column.  Their rows without MUST
are SHOULDs or a recorded decision, and are not listed: NFR-PORT-02 (a
decision), NFR-PERF-03, NFR-DEP-02 and NFR-DOC-04 (SHOULD).

**Keeping it true.**  A handoff that adds a MUST requirement, or changes the
behaviour behind one, updates that row in the same commit.  So does a change
that adds, renames or removes a test named here.  Every test in
`tests/server/` and `tests/browser/` cites at least one requirement or
threat ID in its doc comment; `grep -rn "FR-UI-12" crates/` finds a row's
tests from the code side.

### Functional requirements

| Requirement | L1 unit | L2 server | L3 browser | Otherwise, or not covered |
|---|---|---|---|---|
| FR-UI-01 | `components::form_renders_all_ids_once`, `components::subject_hidden_when_option_false`, `components::attributes::required_fields_carry_both_required_attributes` | — | — | — |
| FR-UI-02 | `config::field_text_substitutes_in_a_translated_label`, `components::the_noscript_message_escapes_label_and_class` | — | `pending::the_submit_button_is_busy_while_sending` | every other string: review of `ContactFormLabels` |
| FR-UI-03 | `config::classes_default_is_all_empty`, `components::the_noscript_message_escapes_label_and_class` | — | — | a hook on every structural element: review |
| FR-UI-04 | — | `validation::field_errors_round_trip_without_javascript`, `validation::a_banner_error_sets_no_field_error`, `delivery::the_success_page_is_applied_when_configured` | `focus::focus_moves_to_the_first_invalid_field`, `token::the_hidden_token_survives_a_failed_submission` | the pending state and inline success: review |
| FR-UI-05 | — | — | `pending::the_submit_button_is_busy_while_sending` | — |
| FR-UI-06 | — | `delivery::the_success_page_is_applied_when_configured` | — | inline success with JavaScript: review |
| FR-UI-07 | — | — | `focus::focus_moves_to_the_first_invalid_field` | — |
| FR-UI-08 | `config::field_text_renders_required_and_line_breaks` | `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-UI-09 | `config::code_text_maps_unexpected_to_delivery_failed` | `delivery::a_delivery_error_reaches_the_client_only_as_delivery_failed`, `routing::a_missing_delivery_context_is_not_configured`, `validation::a_banner_error_sets_no_field_error` | — | — |
| FR-UI-10 | `components::attributes::the_honeypot_is_hidden_from_everyone` | — | — | — |
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
| FR-SUB-09 | — | `delivery::a_delivery_error_reaches_the_client_only_as_delivery_failed`, `logging::no_personal_data_or_secret_is_logged` | — | — |
| FR-SUB-10 | **none** | **none** | — | review: `std::env` appears in `src/` only inside rustdoc examples |
| FR-VAL-01 | `model::empty_name_fails`, `model::newline_in_name_fails`, `model::over_long_name_still_yields_length_code`, `model::newline_in_name_yields_line_breaks_code` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-VAL-02 | `model::invalid_email_fails`, `model::bad_email_yields_format_code`, `model::email::reply_to_addresses_are_accepted`, `model::email::a_single_label_domain_is_a_format_error`, `model::email::an_address_literal_is_a_format_error`, `model::email::an_empty_domain_label_is_a_format_error`, `model::email::a_254_character_address_is_accepted`, `model::email::a_255_character_address_is_a_length_error`, `model::email::a_long_invalid_address_reports_its_length`, `model::email::a_blank_email_keeps_its_format_code` | `validation::each_rule_rejects_with_its_field_code`, `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-VAL-03 | `model::newline_in_subject_fails`, `model::over_long_subject_yields_length_code_with_zero_min`, `model::empty_subject_uses_fallback` | `validation::each_rule_rejects_with_its_field_code`, `validation::a_blank_subject_is_delivered_as_absent` | — | — |
| FR-VAL-04 | `model::too_long_message_fails`, `model::empty_message_yields_required_code`, `model::over_long_message_yields_length_code_at_the_ceiling` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-VAL-05 | `model::honeypot_input_is_detected` | `silent::a_silent_outcome_is_indistinguishable_from_delivery` | — | — |
| FR-VAL-06 | `form_token::issued_token_verifies_once_old_enough`, `form_token::malformed_tokens_are_rejected`, `form_token::an_old_token_is_expired`, `form_token::a_token_younger_than_the_minimum_is_too_young` | `form_token::a_missing_malformed_or_expired_token_is_rejected`, `form_token::a_too_young_token_is_retryable`, `form_token::a_0_4_csrf_token_field_is_no_longer_accepted` | — | — |
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
| FR-ABUSE-10 | `challenge::row_5_passed_proceeds`, `challenge::row_6_not_passed_fails`, `challenge::http::each_provider_uses_its_vendor_endpoint_by_default`, `components::turnstile_renders_its_element_and_script`, `components::hcaptcha_renders_its_element_and_script_and_omits_auto_theme`, `components::recaptcha_v2_renders_its_element_and_script_and_omits_auto_theme`, `components::recaptcha_v3_renders_the_hidden_input_render_url_and_submit_script` | `challenge::challenge_decision_table_rows_1_to_7` | `challenge::a_widget_mounted_after_its_vendor_global_renders_once`, `challenge::a_vendor_script_already_in_head_is_not_inserted_again` | the live vendor contracts: the `#[ignore]`d `live_` tests, by hand; the client-side script insertion (`append_script`) has no browser test, because a negative control would fetch the real vendor script |
| FR-ABUSE-11 | `challenge::row_3_context_no_token_under_reject_is_required`, `challenge::row_4_context_no_token_under_accept_proceeds`, `components::reject_renders_noscript_and_accept_does_not`, `config::no_js_policy_defaults_to_reject` | `challenge::challenge_decision_table_rows_1_to_7` | — | — |
| FR-ABUSE-12 | `challenge::row_7_verifier_error_is_unavailable_for_every_variant`, `challenge::http::a_server_error_is_unavailable`, `challenge::http::a_silent_server_is_a_timeout`, `challenge::http::a_redirect_is_not_followed_and_is_unavailable`, `challenge::http::an_empty_secret_is_misconfigured_and_sends_nothing`, `challenge::http::the_default_timeout_is_five_seconds` | `challenge::challenge_decision_table_rows_1_to_7` | — | — |
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
| FR-I18N-02 | `error::contact_error_code_round_trips_through_the_wire_string`, `config::code_text_maps_unexpected_to_delivery_failed`, `config::field_text_substitutes_min_and_max` | `validation::field_errors_round_trip_without_javascript` | — | — |
| FR-I18N-04 | **none** | **none** | — | review: the rendered markup carries no `dir`, `lang` or locale formatting |
| FR-I18N-05 | `model::message_length_counts_characters`, `delivery::smtp::reply_to_with_special_chars_in_name` | `validation::each_rule_rejects_with_its_field_code` (a non-ASCII message) | — | a non-ASCII value reaching delivery unchanged, and header encoding: review (lettre) |
| FR-A11Y-01 | `components::attributes::every_input_has_a_label_for_it` | — | — | — |
| FR-A11Y-02 | `components::attributes::required_fields_carry_both_required_attributes` | — | — | — |
| FR-A11Y-03 | — | `validation::field_errors_round_trip_without_javascript` | `focus::focus_moves_to_the_first_invalid_field` | — |
| FR-A11Y-04 | — | `validation::a_banner_error_sets_no_field_error` (the assertive banner) | — | the polite success region and field alerts: review |
| FR-A11Y-05 | — | — | `pending::the_submit_button_is_busy_while_sending` | — |
| FR-A11Y-06 | `components::attributes::the_honeypot_is_hidden_from_everyone` | — | — | — |
| FR-A11Y-07 | **none** | **none** | **none** | review: native controls only, and the crate ships no CSS |
| FR-A11Y-08 | **none** | **none** | **none** | review: every state has text; the crate ships no colours |
| FR-A11Y-09 | **none** | **none** | **none** | by construction and review; no automated accessibility audit runs |
| FR-PE-01 | — | `delivery::a_valid_submission_is_delivered_once_in_both_forms` | — | — |
| FR-PE-02 | — | `validation::field_errors_round_trip_without_javascript`, `validation::each_rule_rejects_with_its_field_code` | — | — |
| FR-PE-03 | — | `delivery::the_success_page_is_applied_when_configured` | — | — |
| FR-PE-04 | — | `validation::each_rule_rejects_with_its_field_code`, `policy::server_policy_requires_the_subject`, `policy::server_policy_counts_the_message_in_characters`, `delivery::a_valid_submission_is_delivered_once_in_both_forms` | — | — |
| FR-OBS-01 | — | `logging::no_personal_data_or_secret_is_logged` | — | each event's level: review |
| FR-OBS-02 | `server::pii_not_present_in_expected_log_messages` | `logging::no_personal_data_or_secret_is_logged` | — | — |
| FR-OBS-03 | — | `logging::no_personal_data_or_secret_is_logged` | — | — |
| FR-OBS-04 | **none** | **none** | — | review: no storage in the crate or its dependencies |

### Non-functional requirements

| Requirement | L1 unit | L2 server | L3 browser | Otherwise, or not covered |
|---|---|---|---|---|
| NFR-SEC-01 | **none** | **none** | **none** | the `browser` CI job builds the crate without `ssr`, where the configuration types do not exist; review |
| NFR-SEC-02 | `security::header_injection_attempt_is_sanitised`, `model::newline_in_name_fails`, `model::newline_in_subject_fails` | `validation::each_rule_rejects_with_its_field_code` | — | — |
| NFR-SEC-03 | **none** | **none** | — | review of `constant_time_eq` in `form_token.rs` |
| NFR-SEC-04 | `challenge::row_7_verifier_error_is_unavailable_for_every_variant`, `challenge::http::a_server_error_is_unavailable` | `form_token::a_missing_token_config_fails_closed`, `routing::a_missing_delivery_context_is_not_configured`, `challenge::challenge_decision_table_rows_1_to_7` | — | — |
| NFR-SEC-05 | **none** | **none** | — | review, at each RFC that adds a data flow |
| NFR-SEC-06 | **none** | **none** | — | review of the dependency tree at the release security audit |
| NFR-PRIV-01 | `server::pii_not_present_in_expected_log_messages` | `logging::no_personal_data_or_secret_is_logged` | — | never persisted: see FR-OBS-04 |
| NFR-PRIV-02 | **none** | **none** | — | documentation (per-provider notes) |
| NFR-COMPAT-01 | **none** | **none** | **none** | CI builds: `cargo test --all-features` (the `ssr`, `hydrate` and `islands` features together) and the `browser` job (`hydrate` on wasm32); Islands mode behaviour: review |
| NFR-COMPAT-02 | **none** | **none** | **none** | review: CI runs Rust 1.91, not the MSRV 1.85, so nothing builds on 1.85 until P-24 |
| NFR-COMPAT-03 | **none** | **none** | — | review of `Cargo.toml` (`axum` is optional, behind `axum-helpers`); the `browser` job builds the crate without it |
| NFR-COMPAT-04 | — | `form_token::a_0_4_csrf_token_field_is_no_longer_accepted` | — | the rest of the policy: review at release |
| NFR-COMPAT-05 | **none** | **none** | **none** | review at release; the SSR tests in `components::` pin the ids, field names and the ARIA attributes tested above, so a change to those fails a test |
| NFR-PORT-01 | **none** | **none** | **none** | CI: the `check` job builds `default = []` for wasm32 (`cargo check -p leptos-hl-contact --target wasm32-unknown-unknown`); the `browser` job and the example's wasm check build it with `hydrate` |
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
cargo-leptos.

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
