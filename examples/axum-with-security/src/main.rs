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

#[cfg(feature = "ssr")]
use std::{net::SocketAddr, sync::Arc};

#[cfg(feature = "ssr")]
use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
#[cfg(feature = "ssr")]
use leptos::config::get_configuration;
#[cfg(feature = "ssr")]
use leptos::context::provide_context;
#[cfg(feature = "ssr")]
use leptos_axum::{LeptosRoutes, generate_route_list};
#[cfg(feature = "ssr")]
use tower_http::limit::RequestBodyLimitLayer;
#[cfg(feature = "ssr")]
use tower_governor::{
    GovernorLayer,
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
};
#[cfg(feature = "ssr")]
use url::Url;
#[cfg(feature = "ssr")]
use leptos_hl_contact::{
    axum_helpers::{delivery_context_fn, success_redirect},
    csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token},
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

// Uncomment for real SMTP delivery:
// use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

#[cfg(feature = "ssr")]
use axum_with_security::app::{self, shell};

// ---------------------------------------------------------------------------
// Strict Origin / Referer validation
// ---------------------------------------------------------------------------

#[cfg(feature = "ssr")]
#[derive(Clone)]
struct SecurityState {
    allowed_origin: Arc<Url>,
}

/// Compare the Origin (or Referer) header against the configured allowed origin.
///
/// Parses both values as URLs and compares scheme, host, and port — preventing
/// prefix-spoofing attacks such as `https://example.com.evil.test`.
#[cfg(feature = "ssr")]
fn origin_matches(header_value: &str, allowed: &Url) -> bool {
    let Ok(parsed) = Url::parse(header_value) else {
        return false;
    };
    parsed.scheme() == allowed.scheme()
        && parsed.host_str() == allowed.host_str()
        && parsed.port_or_known_default() == allowed.port_or_known_default()
}

#[cfg(feature = "ssr")]
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
#[cfg(feature = "ssr")]
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

    // Built once, before the router, so an invalid path panics at boot rather
    // than on the first submission.
    let redirect = success_redirect("/thanks");

    // ------------------------------------------------------------------
    // Axum router
    // ------------------------------------------------------------------
    // One context closure.  `leptos_routes_with_context` registers the server
    // functions at their own paths using this same closure, so everything the
    // page render and `submit_contact` need is provided here, once.
    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                ctx.clone()();
                provide_context::<CsrfConfigContext>(Arc::clone(&csrf_config));
                // Unused on server-function requests, which read the submitted
                // token rather than issuing one.
                provide_context(generate_csrf_token(&csrf_config));
                provide_context(redirect.clone());
            },
            {
                let o = leptos_options.clone();
                move || shell(o.clone())
            },
        )
        // Serves the WASM client from `/pkg/*` (site-root in Cargo.toml).
        .fallback(leptos_axum::file_and_error_handler(shell))
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
    // Connection info is what lets `SmartIpKeyExtractor` fall back to the peer
    // address when no forwarded-IP header is present.  Without it every such
    // request fails with "Unable To Extract Key!" (500).
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

// Without `ssr` this crate builds only as a `cdylib` for the browser; the
// binary target still has to compile, so it gets an empty `main`.
#[cfg(not(feature = "ssr"))]
fn main() {}
