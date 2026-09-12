# Axum Integration

## The two context sites

Leptos executes server functions in one Axum handler and renders pages in
another.  They do not share context.  Everything the crate reads from
context must therefore be provided in **both** closures:

| Value | Server-function handler | SSR renderer |
|-------|-------------------------|--------------|
| `ContactDeliveryContext` | required | required |
| `CsrfConfigContext` (`csrf` feature) | required | required |
| `CsrfToken` (`csrf` feature) | — | required, one fresh token per request |
| `ContactServerPolicy` | optional | optional |
| `ContactSuccessRedirect` | required for the redirect | required for the redirect |

Missing a required value does not crash the server: the server function
logs at `error` and answers with a generic "not configured" message.

> Provide every value in **both** closures, even one the table calls
> server-function-only.  `leptos_routes_with_context` registers each server
> function at its own literal path using the SSR closure, and Axum prefers
> that literal path over a `/api/{*fn_name}` wildcard registered by hand — so
> a value provided only in the wildcard's closure can be silently invisible
> to the server function.

## Minimal wiring

```rust,ignore
use std::sync::Arc;
use axum::{Router, body::Body, extract::Request, routing::post};
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

#[tokio::main]
async fn main() {
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);      // one closure, cloned twice

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/{*fn_name}", post({
            let ctx = ctx.clone();
            move |req: Request<Body>| {
                let ctx = ctx.clone();
                async move { handle_server_fns_with_context(ctx, req).await }
            }
        }))
        .leptos_routes_with_context(&leptos_options, routes, ctx, App)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`delivery_context_fn` and `provide_contact_delivery` come from the
`axum-helpers` feature.  Without it, call
`leptos::context::provide_context::<ContactDeliveryContext>(Arc::clone(&d))`
yourself in each closure.

## All context values together

With the `csrf` feature and a server policy the two closures look like this:

```rust,ignore
use leptos::context::provide_context;
use leptos_hl_contact::{
    ContactServerPolicy,
    axum_helpers::success_redirect,
    csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token},
};

let csrf: CsrfConfigContext = Arc::new(CsrfConfig::new(
    std::env::var("CSRF_SECRET").expect("CSRF_SECRET").into_bytes(),
));
let policy = ContactServerPolicy { require_subject: true, max_message_len: 2000 };

let app = Router::new()
    .route("/api/{*fn_name}", post({
        let ctx = ctx.clone();
        let csrf = Arc::clone(&csrf);
        let policy = policy.clone();
        move |req: Request<Body>| {
            let ctx = ctx.clone();
            let csrf = Arc::clone(&csrf);
            let policy = policy.clone();
            async move {
                handle_server_fns_with_context(move || {
                    ctx();
                    provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
                    provide_context(policy.clone());
                    provide_context(success_redirect("/thanks"));
                }, req).await
            }
        }
    }))
    .leptos_routes_with_context(&leptos_options, routes, {
        let csrf = Arc::clone(&csrf);
        move || {
            ctx.clone()();
            provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
            provide_context(generate_csrf_token(&csrf));   // SSR only
            provide_context(policy.clone());
            provide_context(success_redirect("/thanks"));
        }
    }, App)
    .with_state(leptos_options);
```

## Middleware order

Axum applies layers bottom-to-top: the last `.layer()` runs first.  Add the
protective layers after the routes so they cover the server-function
endpoint too:

```rust,ignore
let app = Router::new()
    .route("/api/{*fn_name}", post(/* … */))
    .leptos_routes_with_context(/* … */)
    .with_state(leptos_options)
    .layer(from_fn_with_state(security_state, check_origin))   // runs 3rd
    .layer(GovernorLayer::new(governor_config))                // runs 2nd
    .layer(RequestBodyLimitLayer::new(32 * 1024));             // runs 1st
```

Each layer is explained in [Hardening](../security/hardening.md).  The
complete file is
[`examples/axum-with-security/src/main.rs`](https://github.com/nabbisen/leptos-hl-contact/blob/main/examples/axum-with-security/src/main.rs).

## Other frameworks

The core crate does not depend on Axum.  With Actix Web or another backend,
provide the same context values through that framework's Leptos
integration; the types and the two-site rule are identical.
