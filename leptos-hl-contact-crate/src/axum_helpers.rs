// axum_helpers.rs — Convenience helpers for Axum integration.
//
// Enabled by the `axum-helpers` feature flag.
//
// # Why this module exists
//
// When using Leptos server functions with Axum, the delivery context must be
// provided to **two** places:
//
// 1. The server-function handler (`leptos_axum::handle_server_fns_with_context`)
// 2. The SSR rendering handler (`LeptosRoutes::leptos_routes_with_context`)
//
// The helpers here make it easy to build the context closure without
// repeating the Arc::clone boilerplate in both places.

use std::sync::Arc;

use crate::delivery::ContactDeliveryContext;

// ---------------------------------------------------------------------------
// provide_contact_delivery
// ---------------------------------------------------------------------------

/// Register a [`ContactDeliveryContext`] as a Leptos context value.
///
/// Call this inside the context closure passed to both
/// `handle_server_fns_with_context` and `leptos_routes_with_context`.
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
/// let d1 = Arc::clone(&delivery);
/// let d2 = Arc::clone(&delivery);
///
/// let app = axum::Router::new()
///     .route("/api/*fn_name", post(move |req| {
///         let d = Arc::clone(&d1);
///         handle_server_fns_with_context(move || provide_contact_delivery(Arc::clone(&d)), req)
///     }))
///     .leptos_routes_with_context(&opts, routes,
///         move || provide_contact_delivery(Arc::clone(&d2)), App);
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
/// passing to both `handle_server_fns_with_context` and
/// `leptos_routes_with_context`.  Each invocation of the closure clones the
/// `Arc` and calls `provide_contact_delivery`.
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
///     .route("/api/*fn_name", post({
///         let ctx = ctx.clone();
///         move |req| handle_server_fns_with_context(ctx.clone(), req)
///     }))
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::noop::NoopDelivery;
    use std::sync::Arc;

    #[test]
    fn delivery_context_fn_is_clone() {
        let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
        let ctx = delivery_context_fn(delivery);
        // Should be cloneable (required by both Axum handler sites).
        let _ctx2 = ctx.clone();
    }
}
