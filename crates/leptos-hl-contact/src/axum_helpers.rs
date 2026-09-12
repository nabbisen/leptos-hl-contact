// axum_helpers.rs — Convenience helpers for Axum integration.
//
// Enabled by the `axum-helpers` feature flag.
//
// # Why this module exists
//
// Everything the crate reads from context is provided in the one closure
// passed to `LeptosRoutes::leptos_routes_with_context`, which serves page
// renders and server functions alike.  The helpers here build that closure
// without the `Arc::clone` boilerplate.

use std::sync::Arc;

#[cfg(feature = "form-token")]
use crate::form_token::{
    FormTokenBinding, FormTokenContext, issue_form_token, issue_form_token_with_nonce,
};
use crate::{config::ContactSuccessRedirect, delivery::ContactDeliveryContext};

// ---------------------------------------------------------------------------
// provide_contact_delivery
// ---------------------------------------------------------------------------

/// Register a [`ContactDeliveryContext`] as a Leptos context value.
///
/// Call this inside the context closure passed to
/// `leptos_routes_with_context`.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use leptos_axum::LeptosRoutes;
/// use leptos_hl_contact::{
///     axum_helpers::provide_contact_delivery,
///     delivery::{ContactDeliveryContext, noop::NoopDelivery},
/// };
///
/// let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
///
/// let app = axum::Router::new()
///     .leptos_routes_with_context(&opts, routes,
///         move || provide_contact_delivery(Arc::clone(&delivery)), App);
/// ```
pub fn provide_contact_delivery(delivery: ContactDeliveryContext) {
    leptos::context::provide_context(delivery);
}

// ---------------------------------------------------------------------------
// delivery_context_fn
// ---------------------------------------------------------------------------

/// Build a `move || …` closure that provides the given delivery context.
///
/// Returns an `impl Fn() + Clone + Send + 'static` closure suitable for
/// passing to `leptos_routes_with_context`.  Each invocation clones the `Arc`
/// and calls `provide_contact_delivery`.
///
/// This avoids manual `Arc::clone` repetition at the call site.
///
/// # Example
///
/// ```rust,ignore
/// use leptos_hl_contact::{
///     axum_helpers::delivery_context_fn,
///     delivery::{ContactDeliveryContext, noop::NoopDelivery},
/// };
/// use std::sync::Arc;
///
/// let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
/// let ctx = delivery_context_fn(delivery);
///
/// let app = axum::Router::new()
///     .leptos_routes_with_context(&opts, routes, ctx, App);
/// ```
pub fn delivery_context_fn(
    delivery: ContactDeliveryContext,
) -> impl Fn() + Clone + Send + Sync + 'static {
    move || {
        let d = Arc::clone(&delivery);
        provide_contact_delivery(d);
    }
}

// ---------------------------------------------------------------------------
// success_redirect
// ---------------------------------------------------------------------------

/// Build a [`ContactSuccessRedirect`] that redirects with
/// [`leptos_axum::redirect`].
///
/// Provide the result via Leptos context in the closure passed to
/// `leptos_routes_with_context`.  Build it once, before the router, so an
/// invalid path panics at startup rather than on the first submission.
///
/// # Panics
///
/// Panics when `path` is not site-relative.  This is startup configuration
/// read from your own source or environment, so a bad value should stop the
/// process rather than silently disable the redirect.  Use
/// [`ContactSuccessRedirect::new`] directly to handle the error yourself.
///
/// # Example
///
/// ```rust,ignore
/// use leptos::context::provide_context;
/// use leptos_hl_contact::axum_helpers::success_redirect;
///
/// // Before the router:
/// let redirect = success_redirect("/thanks");
///
/// // In the context closure:
/// provide_context(redirect.clone());
/// ```
pub fn success_redirect(path: impl Into<String>) -> ContactSuccessRedirect {
    let path = path.into();
    ContactSuccessRedirect::new(path.clone(), leptos_axum::redirect)
        .unwrap_or_else(|e| panic!("success_redirect({path:?}): {e}"))
}

// ---------------------------------------------------------------------------
// Cookie binding — compiled only when `form-token` is active.
// ---------------------------------------------------------------------------

/// How the form token's binding cookie is written.
///
/// The cookie carries the token's nonce, never the token or the secret, and
/// is always `HttpOnly` and `SameSite=Lax`: a cross-site form cannot read it
/// and the browser does not send it on a cross-site POST.
///
/// At the defaults the cookie is sent as `__Host-hl_contact_ft`.  Browsers
/// refuse to store a `__Host-` cookie that names a `Domain`, so a sibling
/// subdomain cannot plant a value of its own choosing.  The prefix is added
/// only when `secure` is `true` and `path` is `/`, which are the conditions
/// the prefix itself requires.
///
/// This is defence in depth.  Origin validation remains the control that
/// rejects cross-site POSTs.
#[cfg(feature = "form-token")]
#[derive(Clone, Debug)]
pub struct FormTokenCookie {
    /// Cookie name, without any prefix.  Defaults to `hl_contact_ft`.
    ///
    /// `__Host-` is prepended automatically when `secure` is `true` and
    /// `path` is `/`.
    pub name: String,
    /// Add the `Secure` attribute.  Defaults to `true`; set `false` only to
    /// develop over plain HTTP.
    pub secure: bool,
    /// Cookie path.  Defaults to `/`.
    pub path: String,
}

#[cfg(feature = "form-token")]
impl Default for FormTokenCookie {
    fn default() -> Self {
        Self {
            name: "hl_contact_ft".into(),
            secure: true,
            path: "/".into(),
        }
    }
}

/// The name the cookie is actually written and read under.
///
/// `__Host-` is legal only on a `Secure` cookie at `Path=/` with no `Domain`;
/// we never send `Domain`, so the other two decide.  Writing and reading both
/// go through here, so the two can never disagree.
#[cfg(feature = "form-token")]
fn effective_name(cookie: &FormTokenCookie) -> String {
    const HOST_PREFIX: &str = "__Host-";

    if cookie.secure && cookie.path == "/" && !cookie.name.starts_with(HOST_PREFIX) {
        format!("{HOST_PREFIX}{}", cookie.name)
    } else {
        cookie.name.clone()
    }
}

/// Build the `Set-Cookie` value for `nonce`.
///
/// Separated from the request handling so the exact string is testable.
#[cfg(feature = "form-token")]
fn set_cookie_value(nonce: &str, cookie: &FormTokenCookie, max_age: u64) -> String {
    let mut v = format!(
        "{}={}; HttpOnly; SameSite=Lax; Path={}; Max-Age={}",
        effective_name(cookie),
        nonce,
        cookie.path,
        max_age
    );
    if cookie.secure {
        v.push_str("; Secure");
    }
    v
}

/// The nonce is the token's middle segment.
#[cfg(feature = "form-token")]
fn token_nonce(token: &str) -> Option<&str> {
    token.split('|').nth(1)
}

/// Find one cookie's value in a `Cookie` header.
///
/// Matches the whole name, so `hl_contact_ft2` never satisfies a lookup for
/// `hl_contact_ft`.  The first match wins.
#[cfg(feature = "form-token")]
fn cookie_value(header: &str, name: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k.trim() == name).then(|| v.trim().to_owned())
    })
}

/// Issue a token for a page render and set its binding cookie.
///
/// Call this inside the closure passed to `leptos_routes_with_context`.  It
/// does nothing unless the request is a `GET`: the same closure serves page
/// renders and server functions (see the Axum Integration guide), and a POST
/// response must not overwrite the cookie the submitted form was bound to.
///
/// Provides [`FormToken`](crate::form_token::FormToken) so `ContactForm` can
/// render the hidden field, and
/// appends a `Set-Cookie` header carrying the token's nonce.
///
/// The nonce is **reused** when the request already carries a usable cookie,
/// and only minted when it does not.  The cookie identifies the browser, so
/// it has to survive the visitor opening a second tab or walking to another
/// page and back; a fresh nonce per render would overwrite it and invalidate
/// every form already on screen.  The header is sent either way, which keeps
/// `Max-Age` refreshed while the visitor browses.  The token itself is new on
/// every render, with its own timestamp, TTL and minimum age.
#[cfg(feature = "form-token")]
pub fn provide_form_token_with_cookie(config: &FormTokenContext, cookie: &FormTokenCookie) {
    use leptos::context::{provide_context, use_context};

    let Some(parts) = use_context::<axum::http::request::Parts>() else {
        return;
    };
    if parts.method != axum::http::Method::GET {
        return;
    }

    // A cookie that is absent, truncated or not hex is not trusted: the
    // helper mints a nonce instead of signing whatever arrived.
    let token = request_cookie(&parts, &effective_name(cookie))
        .and_then(|nonce| issue_form_token_with_nonce(config, &nonce))
        .unwrap_or_else(|| issue_form_token(config));

    if let Some(nonce) = token_nonce(&token.0)
        && let Some(res) = use_context::<leptos_axum::ResponseOptions>()
        && let Ok(value) =
            axum::http::HeaderValue::from_str(&set_cookie_value(nonce, cookie, config.ttl_secs))
    {
        // Append, so a cookie set elsewhere in the response survives.
        res.append_header(axum::http::header::SET_COOKIE, value);
    }

    provide_context(token);
}

/// Read the binding cookie from the request and provide it as
/// [`FormTokenBinding`].
///
/// Call this inside the same context closure.  A missing request, header or
/// cookie all yield `FormTokenBinding(None)`, which a `Binding::Cookie`
/// configuration rejects as `BindingMissing`.
#[cfg(feature = "form-token")]
pub fn provide_form_token_binding(cookie: &FormTokenCookie) {
    use leptos::context::{provide_context, use_context};

    let value = use_context::<axum::http::request::Parts>()
        .and_then(|parts| request_cookie(&parts, &effective_name(cookie)));

    provide_context(FormTokenBinding(value));
}

/// The named cookie as it arrived on this request, if it did.
#[cfg(feature = "form-token")]
fn request_cookie(parts: &axum::http::request::Parts, name: &str) -> Option<String> {
    parts
        .headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| cookie_value(h, name))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
