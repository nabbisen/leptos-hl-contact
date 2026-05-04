// examples/axum-basic/src/main.rs
//
// Demonstrates how to integrate leptos-hl-contact with an Axum server.
//
// SECURITY NOTICE:
//   - Load all SMTP credentials from environment variables.
//   - Never hard-code passwords or API keys in source code.
//   - Enable HTTPS (TLS termination) in production.
//   - Add rate limiting middleware before exposing this publicly.
//
// Run:
//   SMTP_HOST=smtp.example.com \
//   SMTP_USER=you@example.com \
//   SMTP_PASS=secret \
//   SMTP_FROM=noreply@example.com \
//   CONTACT_TO=you@example.com \
//   cargo run -p axum-basic

use std::sync::Arc;

use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};

// Uncomment and configure to use real SMTP delivery:
// use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

mod app;

#[tokio::main]
async fn main() {
    // Initialise logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,leptos=debug".into()),
        )
        .init();

    // Load .env file if present (for local development only).
    let _ = dotenvy::dotenv();

    // ------------------------------------------------------------------
    // Build the delivery backend.
    //
    // For production, replace NoopDelivery with LettreSmtpDelivery:
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

    // Read Leptos config from the environment / leptos.toml.
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(app::App);

    // Clone the delivery context so it can be moved into both closures below.
    let delivery_for_server_fns = Arc::clone(&delivery);

    let app = Router::new()
        // Server function handler — delivery context must be provided here.
        .route(
            "/api/*fn_name",
            axum::routing::post(move |req| {
                let d = Arc::clone(&delivery_for_server_fns);
                handle_server_fns_with_context(
                    move || {
                        leptos::context::provide_context(Arc::clone(&d));
                    },
                    req,
                )
            }),
        )
        // SSR page renderer — delivery context must also be provided here.
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                leptos::context::provide_context(Arc::clone(&delivery));
            },
            app::App,
        )
        .with_state(leptos_options);

    tracing::info!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
