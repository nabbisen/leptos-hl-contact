// examples/axum-with-security/src/main.rs
//
// Production-ready integration of leptos-hl-contact with:
//   - Request body size limit (32 KiB) — prevents large-POST abuse
//   - Rate limiting via tower_governor (IP-based, 2 req/s, burst 5, HTTP 429)
//   - CSRF token verification via HMAC-SHA256 (stateless, no session needed)
//   - Strict Origin / Referer validation (URL-parsed, scheme+host+port compared)
//
// Required environment variables:
//   CSRF_SECRET=<openssl rand -hex 32>    # 32+ random bytes; NO default fallback
//   ALLOWED_ORIGIN=https://example.com   # origin URL; NO default fallback
//   SMTP_HOST / SMTP_USER / SMTP_PASS / SMTP_FROM / CONTACT_TO  (for real SMTP)
//
// SECURITY NOTICE:
//   - Always run behind HTTPS in production; update ALLOWED_ORIGIN accordingly.
//   - Set CSRF_SECRET to a unique, random value per deployment.
//   - Ensure your reverse proxy validates X-Forwarded-For before reaching Axum.

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
use tower_http::limit::RequestBodyLimitLayer;
use tower_governor::{
    GovernorLayer,
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
};
use url::Url;
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token},
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

// Uncomment for real SMTP delivery:
// use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

mod app;

// ---------------------------------------------------------------------------
// Strict Origin / Referer validation
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SecurityState {
    allowed_origin: Arc<Url>,
}

/// Compare the Origin (or Referer) header against the configured allowed origin.
///
/// Parses both values as URLs and compares scheme, host, and port — preventing
/// prefix-spoofing attacks such as `https://example.com.evil.test`.
fn origin_matches(header_value: &str, allowed: &Url) -> bool {
    let Ok(parsed) = Url::parse(header_value) else {
        return false;
    };
    parsed.scheme() == allowed.scheme()
        && parsed.host_str() == allowed.host_str()
        && parsed.port_or_known_default() == allowed.port_or_known_default()
}

async fn check_origin(
    axum::extract::State(state): axum::extract::State<SecurityState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::POST {
        let value = req
            .headers()
            .get(header::ORIGIN)
            .or_else(|| req.headers().get(header::REFERER))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !origin_matches(value, &state.allowed_origin) {
            tracing::warn!(
                header_value = value,
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
    // CSRF configuration — fail-closed if CSRF_SECRET is missing
    // ------------------------------------------------------------------
    // CSRF_SECRET is required.  Generate with: openssl rand -hex 32
    let csrf_secret = std::env::var("CSRF_SECRET")
        .expect(
            "CSRF_SECRET must be set to a 32+ byte random value. \
             Generate one with: openssl rand -hex 32",
        )
        .into_bytes();

    let csrf_config: CsrfConfigContext = Arc::new(CsrfConfig {
        secret_key:     csrf_secret,
        token_ttl_secs: 3600,
    });

    // ------------------------------------------------------------------
    // Delivery backend
    // ------------------------------------------------------------------
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);

    // ------------------------------------------------------------------
    // Rate limiting
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
    // Strict origin validation
    // ------------------------------------------------------------------
    // ALLOWED_ORIGIN is required in production (e.g. https://example.com).
    let allowed_origin_str = std::env::var("ALLOWED_ORIGIN")
        .expect("ALLOWED_ORIGIN must be set (e.g. https://example.com)");
    let allowed_origin = Url::parse(&allowed_origin_str)
        .unwrap_or_else(|e| panic!("ALLOWED_ORIGIN is not a valid URL: {e}"));
    let security_state = SecurityState {
        allowed_origin: Arc::new(allowed_origin),
    };

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
        // Server function handler: delivery context + CSRF config for verification.
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
        // SSR renderer: delivery context + CSRF config + fresh per-request token.
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                ctx.clone()();
                provide_context::<CsrfConfigContext>(Arc::clone(&csrf_for_ssr));
                provide_context(generate_csrf_token(&csrf_for_ssr));
            },
            app::App,
        )
        .with_state(leptos_options)
        // Security layers (outermost runs first):
        .layer(from_fn_with_state(security_state, check_origin))
        .layer(GovernorLayer::new(governor_config))
        // 32 KiB body limit — prevents large-POST abuse before any handler runs.
        .layer(RequestBodyLimitLayer::new(32 * 1024));

    tracing::info!(
        addr = %addr,
        "leptos-hl-contact (body-limit + rate-limit + CSRF + Origin validation)"
    );
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
