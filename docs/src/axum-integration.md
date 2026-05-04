# Axum Integration

This page explains how to wire `leptos-hl-contact` into an Axum application.

## Context injection — the key concept

Leptos server functions run in a separate HTTP handler from the SSR renderer.
Both need access to the `ContactDeliveryContext` (the delivery backend).

You must provide the context in **two** places:

1. The server-function handler (`handle_server_fns_with_context`)
2. The SSR renderer (`leptos_routes_with_context`)

If you forget either one, the server function will log an error and return a
generic "not configured" message.

## Using `delivery_context_fn` (recommended)

Enable the `axum-helpers` feature and use `delivery_context_fn` to build a
reusable closure that clones the `Arc` automatically:

```toml
# Cargo.toml
leptos-hl-contact = { version = "0.2", features = ["ssr", "smtp-lettre", "axum-helpers"] }
```

```rust,ignore
use std::sync::Arc;
use axum::{Router, routing::post};
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

#[tokio::main]
async fn main() {
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);   // <-- build once, clone twice

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/*fn_name", post({
            let ctx = ctx.clone();
            move |req| handle_server_fns_with_context(ctx.clone(), req)
        }))
        .leptos_routes_with_context(&leptos_options, routes, ctx, App)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Manual injection (without `axum-helpers`)

If you prefer not to enable the helper feature, inject the context manually:

```rust,ignore
use leptos_hl_contact::delivery::ContactDeliveryContext;

let d_server = Arc::clone(&delivery);
let d_ssr    = Arc::clone(&delivery);

let app = Router::new()
    .route("/api/*fn_name", post(move |req| {
        let d = Arc::clone(&d_server);
        handle_server_fns_with_context(
            move || leptos::context::provide_context::<ContactDeliveryContext>(Arc::clone(&d)),
            req,
        )
    }))
    .leptos_routes_with_context(&leptos_options, routes, move || {
        leptos::context::provide_context::<ContactDeliveryContext>(Arc::clone(&d_ssr));
    }, App)
    .with_state(leptos_options);
```

## Placing `ContactForm` in your app

```rust,ignore
use leptos::prelude::*;
use leptos_hl_contact::ContactForm;

#[component]
fn ContactPage() -> impl IntoView {
    view! {
        <main>
            <h1>"Contact us"</h1>
            <ContactForm />
        </main>
    }
}
```

## Security middleware order

Layer order in Axum is **bottom-to-top** (last added = outermost).
Add rate limiting and CSRF middleware **after** the routes so they apply to
all requests including server functions:

```rust,ignore
let app = Router::new()
    .route("/api/*fn_name", post(...))
    .leptos_routes_with_context(...)
    .with_state(leptos_options)
    // Applied outermost — runs first:
    .layer(GovernorLayer { config: governor_config })
    .layer(from_fn(check_origin));
```

See [Security](./security.md) for the complete checklist.
