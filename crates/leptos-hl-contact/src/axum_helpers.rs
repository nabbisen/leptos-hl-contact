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
/// use axum::routing::post;
/// use leptos_axum::{handle_server_fns_with_context, LeptosRoutes};
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
