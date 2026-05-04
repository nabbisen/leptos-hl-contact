// examples/axum-basic/src/main.rs
//
// Demonstrates how to integrate leptos-hl-contact with an Axum 0.8 server.
//
// SECURITY NOTICE:
//   - Load all SMTP credentials from environment variables — never hard-code them.
//   - Enable HTTPS (TLS termination) in production.
//   - Add rate-limiting middleware before exposing this publicly.
//   - Set SameSite=Lax or Strict cookies to mitigate CSRF risk.
//
// Run with NoopDelivery (discards submissions — safe for local dev):
//   cargo run -p axum-basic
//
// Run with real SMTP:
//   SMTP_HOST=smtp.example.com \
//   SMTP_USER=you@example.com  \
//   SMTP_PASS=secret           \
//   SMTP_FROM=noreply@example.com \
//   CONTACT_TO=inbox@example.com  \
//   cargo run -p axum-basic

use std::sync::Arc;

use axum::{Router, body::Body, extract::Request, routing::post};
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

// Uncomment to use real SMTP delivery (requires smtp-lettre feature):
// use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

mod app;

#[tokio::main]
async fn main() {
    // Initialise tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,leptos=debug".into()),
        )
        .init();

    // Load .env file if present (local dev only — not for production).
    let _ = dotenvy::dotenv();

    // ------------------------------------------------------------------
    // Build the delivery backend.
    //
    // NoopDelivery discards all submissions. Replace with LettreSmtpDelivery
    // for production:
    //
    //   let delivery: ContactDeliveryContext = Arc::new(LettreSmtpDelivery {
    //       config: SmtpConfig {
    //           host:           std::env::var("SMTP_HOST").expect("SMTP_HOST"),
    //           port:           587,
    //           username:       std::env::var("SMTP_USER").expect("SMTP_USER"),
    //           password:       std::env::var("SMTP_PASS").expect("SMTP_PASS"),
    //           from_address:   std::env::var("SMTP_FROM").expect("SMTP_FROM"),
    //           to_address:     std::env::var("CONTACT_TO").expect("CONTACT_TO"),
    //           subject_prefix: "[Contact]".into(),
    //           tls_mode:       SmtpTlsMode::StartTls,
    //       },
    //   });
    // ------------------------------------------------------------------
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);

    // Build a reusable context closure.
    // delivery_context_fn returns an Arc-cloning Fn() closure that can be
    // passed to both Axum handler sites without manual Arc::clone repetition.
    let ctx = delivery_context_fn(delivery);

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(app::App);

    let app = Router::new()
        // Server function handler — delivery context provided here.
        // In Axum 0.8 the closure must be explicitly async.
        .route(
            "/api/*fn_name",
            post({
                let ctx = ctx.clone();
                move |req: Request<Body>| {
                    let ctx = ctx.clone();
                    async move { handle_server_fns_with_context(ctx, req).await }
                }
            }),
        )
        // SSR renderer — delivery context must also be provided here.
        .leptos_routes_with_context(&leptos_options, routes, ctx, app::App)
        .with_state(leptos_options);

    tracing::info!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
