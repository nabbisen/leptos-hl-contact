// examples/axum-with-security/src/main.rs
//
// Demonstrates a production-ready integration of leptos-hl-contact with:
//   - Rate limiting via `tower_governor` (IP-based leaky-bucket, HTTP 429)
//   - CSRF protection via leptos-hl-contact's built-in HMAC-SHA256 token helper
//   - Origin header validation middleware
//
// SECURITY NOTICE:
//   - Load all secrets from environment variables; never hard-code them.
//   - Enable HTTPS (TLS termination) in production; update ALLOWED_ORIGIN.
//   - The CSRF_SECRET must be at least 32 random bytes, server-side only.
//     Generate one: openssl rand -hex 32
//
// Run (local dev, NoopDelivery):
//   CSRF_SECRET=change-me-use-32-random-bytes-in-prod \
//   ALLOWED_ORIGIN=http://localhost:3000 \
//   cargo run -p axum-with-security

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use leptos::config::get_configuration;
use leptos::context::provide_context;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use tower_governor::{
    GovernorLayer,
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
};
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token},
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

// Uncomment for real SMTP:
// use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

mod app;

// ---------------------------------------------------------------------------
// Application state shared across middleware
// ---------------------------------------------------------------------------
#[derive(Clone)]
struct SecurityState {
    allowed_origin: String,
}

// ---------------------------------------------------------------------------
// Origin / Referer check middleware
//
// Rejects POST requests whose Origin or Referer header does not start with
// `allowed_origin`.  This is a first line of defence against CSRF; the
// HMAC token provides a second, independent layer.
//
// In production, set ALLOWED_ORIGIN to your actual domain, e.g.:
//   https://example.com
// ---------------------------------------------------------------------------
async fn check_origin(
    axum::extract::State(state): axum::extract::State<SecurityState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::POST {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .or_else(|| req.headers().get(header::REFERER))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !origin.starts_with(&state.allowed_origin) {
            tracing::warn!(
                origin,
                allowed = %state.allowed_origin,
                "rejected POST: origin mismatch"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,leptos=debug,tower_governor=info".into()),
        )
        .init();

    let _ = dotenvy::dotenv();

    // ------------------------------------------------------------------
    // CSRF configuration
    //
    // The secret key signs every CSRF token.  It must stay server-side.
    // Minimum recommended length: 32 bytes.
    //   openssl rand -hex 32
    // ------------------------------------------------------------------
    let csrf_secret = std::env::var("CSRF_SECRET")
        .unwrap_or_else(|_| {
            tracing::warn!(
                "CSRF_SECRET not set — using an insecure default. \
                 Set a 32+ byte random secret in production!"
            );
            "insecure-placeholder-change-before-deploying-this-app".into()
        })
        .into_bytes();

    let csrf_config: CsrfConfigContext = Arc::new(CsrfConfig {
        secret_key:     csrf_secret,
        token_ttl_secs: 3600, // tokens valid for 1 hour
    });

    // ------------------------------------------------------------------
    // Delivery backend
    //
    // NoopDelivery discards submissions — suitable for local development.
    // Replace with LettreSmtpDelivery for production.
    // ------------------------------------------------------------------
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);

    // ------------------------------------------------------------------
    // Rate limiting — tower_governor
    //
    // SmartIpKeyExtractor reads the real IP from X-Forwarded-For or the
    // peer address.  In production, ensure your reverse proxy sets and
    // validates X-Forwarded-For before traffic reaches Axum.
    //
    // Settings: 2 requests/second sustained; burst of 5.
    // Exceeding the limit returns HTTP 429 Too Many Requests automatically.
    // ------------------------------------------------------------------
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(2)
            .burst_size(5)
            .finish()
            .expect("valid governor config"),
    );

    // ------------------------------------------------------------------
    // Origin validation
    // ------------------------------------------------------------------
    let allowed_origin = std::env::var("ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".into());
    let security_state = SecurityState { allowed_origin };

    // ------------------------------------------------------------------
    // Leptos configuration
    // ------------------------------------------------------------------
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(app::App);

    let csrf_for_server_fns = Arc::clone(&csrf_config);
    let csrf_for_ssr         = Arc::clone(&csrf_config);

    // ------------------------------------------------------------------
    // Axum router
    // ------------------------------------------------------------------
    let app = Router::new()
        // ── Server function handler ──────────────────────────────────────
        // Inject: delivery context + CSRF config (for token verification).
        // Do NOT inject a CsrfToken here — tokens are for page rendering,
        // not for server-function calls.
        .route(
            "/api/*fn_name",
            post({
                let ctx  = ctx.clone();
                let csrf = Arc::clone(&csrf_for_server_fns);
                move |req: Request<Body>| {
                    let ctx  = ctx.clone();
                    let csrf = Arc::clone(&csrf);
                    async move {
                        handle_server_fns_with_context(
                            move || {
                                ctx();
                                provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
                            },
                            req,
                        )
                        .await
                    }
                }
            }),
        )
        // ── SSR renderer ─────────────────────────────────────────────────
        // Inject: delivery context + CSRF config + a fresh per-request token.
        // Leptos creates a new context per request, so each page load gets a
        // unique token automatically embedded in the hidden form field by
        // ContactForm.
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                ctx.clone()();
                provide_context::<CsrfConfigContext>(Arc::clone(&csrf_for_ssr));
                // generate_csrf_token creates a new HMAC-signed token for this
                // render.  ContactForm reads CsrfToken from context and inserts
                // it into a hidden <input name="csrf_token"> field.
                let token = generate_csrf_token(&csrf_for_ssr);
                provide_context(token);
            },
            app::App,
        )
        .with_state(leptos_options)
        // ── Middleware layers (outermost = first to run) ─────────────────
        // Origin check runs before rate limiting so forged-origin requests
        // are rejected cheaply without consuming a rate-limit slot.
        .layer(from_fn_with_state(security_state, check_origin))
        .layer(GovernorLayer::new(governor_config));

    tracing::info!(
        addr = %addr,
        "leptos-hl-contact secured example ready \
         (rate-limiting + CSRF + Origin validation)"
    );
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
