// examples/axum-basic/src/main.rs
//
// ╔══════════════════════════════════════════════════════════════════╗
// ║  LOCAL DEVELOPMENT ONLY                                          ║
// ║  This example has no rate limiting, no CSRF protection, and no  ║
// ║  Origin validation.  Do NOT deploy it as-is to production.      ║
// ║  Use examples/axum-with-security for production-ready wiring.   ║
// ╚══════════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p axum-basic

use std::sync::Arc;

use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::limit::RequestBodyLimitLayer;
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

mod app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,leptos=debug".into()),
        )
        .init();

    // Loud startup warning so this is never accidentally used in production.
    tracing::warn!(
        "axum-basic is a LOCAL DEVELOPMENT example only. \
         It has no rate limiting, CSRF protection, or Origin validation. \
         Use examples/axum-with-security for production-ready wiring."
    );

    let _ = dotenvy::dotenv();

    // NoopDelivery discards all submissions — safe for local development.
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);

    // `Some("Cargo.toml")` reads [package.metadata.leptos]; `None` reads only
    // environment variables and prints a LEPTOS_OUTPUT_NAME notice without them.
    let conf = get_configuration(Some("Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(app::App);

    // One context closure: `leptos_routes_with_context` registers the server
    // functions at their own paths using this same closure, so the delivery
    // context reaches both the page render and `submit_contact`.
    let app = Router::new()
        .leptos_routes_with_context(&leptos_options, routes, ctx, app::App)
        .with_state(leptos_options)
        // Limit request body size to 32 KiB to prevent large-POST abuse.
        .layer(RequestBodyLimitLayer::new(32 * 1024));

    tracing::info!("Listening on http://{addr} (local dev — not production safe)");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
