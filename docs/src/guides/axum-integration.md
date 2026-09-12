# Axum Integration

## The context closure

Provide everything the crate reads from context in the one closure you pass
to `leptos_routes_with_context`:

| Value | Needed for |
|-------|------------|
| `ContactDeliveryContext` | required — `submit_contact` delivers through it |
| `CsrfConfigContext` (`csrf` feature) | required — token verification, fail-closed without it |
| `CsrfToken` (`csrf` feature) | required — one fresh token per page render; unused on server-function requests |
| `ContactServerPolicy` | optional — server-side limits |
| `ContactSuccessRedirect` | required for the success redirect |

Missing a required value does not crash the server: the server function
logs at `error` and answers with a generic "not configured" message.

> **Why one closure.**  `leptos_routes_with_context` registers each server
> function at its own path using the same closure it renders pages with, so
> a hand-written `/api/{*fn_name}` route is unnecessary — and worse than
> unnecessary: Axum prefers the literal path, so a context value provided
> only on the wildcard route never reaches the server function.  Earlier
> versions of this guide described two context sites; that was wrong.

## Minimal wiring

```rust,ignore
use std::sync::Arc;
use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list};
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, noop::NoopDelivery},
};

#[tokio::main]
async fn main() {
    let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
    let ctx = delivery_context_fn(delivery);

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(&leptos_options, routes, ctx, App)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`delivery_context_fn` and `provide_contact_delivery` come from the
`axum-helpers` feature.  Without it, call
`leptos::context::provide_context::<ContactDeliveryContext>(Arc::clone(&d))`
yourself in the closure.

## All context values together

With the `csrf` feature, a server policy and a success page:

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
// Built before the router so an invalid path panics at boot.
let redirect = success_redirect("/thanks");

let app = Router::new()
    .leptos_routes_with_context(&leptos_options, routes, move || {
        ctx.clone()();
        provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
        // Unused on server-function requests, which read the submitted token
        // rather than issuing one.
        provide_context(generate_csrf_token(&csrf));
        provide_context(policy.clone());
        provide_context(redirect.clone());
    }, App)
    .with_state(leptos_options);
```

### Advanced: excluding a server function

`leptos_routes_with_context` skips any path you list as excluded, which is
the case for registering a server function on a route of your own — a
different middleware stack for one endpoint, say.  If you do that, provide
the context in *that* handler as well: it no longer shares the closure
above.  Unless you need this, one closure is the whole story.

## Middleware order

Axum applies layers bottom-to-top: the last `.layer()` runs first.  Add the
protective layers after the routes so they cover the server-function
endpoint too:

```rust,ignore
let app = Router::new()
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
integration.  The types are identical; check how that integration registers
server functions, since the one-closure rule above is a property of
`leptos_axum`.
