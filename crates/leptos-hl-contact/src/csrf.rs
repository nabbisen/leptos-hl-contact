// csrf.rs — Deprecated aliases for the 0.4 names.
//
// The feature, module and items were renamed in 0.5 because the token is not
// a CSRF control on its own; see `form_token`.  Everything here forwards and
// is removed in the next minor.

use crate::form_token::{FormToken, FormTokenConfig, FormTokenContext, verify_form_token};

/// Renamed to [`FormTokenConfig`](crate::form_token::FormTokenConfig).
///
/// Note that a `CsrfConfig { secret_key, token_ttl_secs }` struct literal no
/// longer compiles: the field is now `ttl_secs` and two more were added.
/// Build the value with `FormTokenConfig::new(secret)` and its `with_*`
/// methods.
#[deprecated(
    since = "0.5.0",
    note = "renamed to `form_token`; removed in the next minor"
)]
pub type CsrfConfig = FormTokenConfig;

/// Renamed to [`FormToken`](crate::form_token::FormToken).
#[deprecated(
    since = "0.5.0",
    note = "renamed to `form_token`; removed in the next minor"
)]
pub type CsrfToken = FormToken;

/// Renamed to [`FormTokenContext`](crate::form_token::FormTokenContext).
#[deprecated(
    since = "0.5.0",
    note = "renamed to `form_token`; removed in the next minor"
)]
pub type CsrfConfigContext = FormTokenContext;

/// Renamed to [`issue_form_token`](crate::form_token::issue_form_token).
#[deprecated(
    since = "0.5.0",
    note = "renamed to `form_token::issue_form_token`; removed in the next minor"
)]
pub fn generate_csrf_token(config: &FormTokenConfig) -> FormToken {
    crate::form_token::issue_form_token(config)
}

/// Renamed to [`verify_form_token`](crate::form_token::verify_form_token),
/// which reports *why* a token failed.
///
/// This wrapper passes no bound value and flattens the result to a `bool`, so
/// a caller cannot tell a too-young token from a forged one.
#[deprecated(
    since = "0.5.0",
    note = "renamed to `form_token::verify_form_token`, which returns the reason; removed in the next minor"
)]
pub fn verify_csrf_token(token: &str, config: &FormTokenConfig) -> bool {
    verify_form_token(token, None, config).is_ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
