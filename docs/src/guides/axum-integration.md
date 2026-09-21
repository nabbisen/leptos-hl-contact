# Axum Integration

## The context closure

Provide everything the crate reads from context in the one closure you pass
to `leptos_routes_with_context`:

| Value | Needed for |
|-------|------------|
| `ContactDeliveryContext` | required — `submit_contact` delivers through it |
| `FormTokenContext` (`form-token` feature) | required — token verification, fail-closed without it |
| `FormToken` (`form-token` feature) | required — one fresh token per page render; unused on server-function requests |
| `FormTokenBinding` (`form-token` feature) | required for `Binding::Cookie` — the cookie as it arrived |
| `FormTokenIssuer` (`form-token` feature) | required for `Binding::Cookie` with a hydrated form — sets the cookie for a token the browser fetches |
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

With the `form-token` feature, a server policy and a success page:

```rust,ignore
use leptos::context::provide_context;
use leptos_hl_contact::{
    ContactServerPolicy,
    axum_helpers::{
        FormTokenCookie, provide_form_token_binding, provide_form_token_issuer,
        provide_form_token_with_cookie, success_redirect,
    },
    form_token::{Binding, FormTokenConfig, FormTokenContext},
};

let token_config: FormTokenContext = Arc::new(
    FormTokenConfig::new(
        std::env::var("FORM_TOKEN_SECRET").expect("FORM_TOKEN_SECRET").into_bytes(),
    )
    .with_binding(Binding::Cookie),
);
let token_cookie = FormTokenCookie::default();
let policy = ContactServerPolicy {
    require_subject: true,
    max_message_len: 2000,
    ..Default::default()
};
// Built before the router so an invalid path panics at boot.
let redirect = success_redirect("/thanks");

let app = Router::new()
    .leptos_routes_with_context(&leptos_options, routes, move || {
        ctx.clone()();
        provide_context::<FormTokenContext>(Arc::clone(&token_config));
        // Issues the token and sets its cookie on GET only.
        provide_form_token_with_cookie(&token_config, &token_cookie);
        // Reads that cookie back on the submission.
        provide_form_token_binding(&token_cookie);
        // Sets it for a token the browser fetches from /api/form_token.
        provide_form_token_issuer(&token_cookie);
        provide_context(policy.clone());
        provide_context(redirect.clone());
    }, App)
    .with_state(leptos_options);
```

The token helpers need the `form-token` feature alongside `axum-helpers`.
Without binding, replace them with
`provide_context(issue_form_token(&token_config))` — see
[Form Token](../security/form-token.md#cookie-binding) for the trade-off.

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

## The error redirect without JavaScript

Without JavaScript the form posts natively, and a server function answers
with a redirect instead of data.

**A refused submission.**
- **The redirect.**  It is answered with `302 Found` back to the page it came
  from (the `Referer`).
- **Two query parameters** are added by Leptos's server functions:
  - `__err`: the error, URL-safe base64;
  - `__path`: the server function's path.
- **The page reads `__err` on load** to show the banner or the field
  errors.

**A successful submission** comes back to that page without them, unless a
success page is set.

**Let the parameters through.**  Middleware, caching and analytics that match
on URLs must let these parameters through:
- a cache keyed without the query string, or a rule that strips unknown
  parameters, loses the message;
- a rule that rejects unknown parameters turns a validation error into a
  broken page.

**Sources.**
- `server_fn` 0.8.12 `src/error.rs` appends `__path` and `__err`.
- `leptos_router` 0.8.13 `src/location/mod.rs` reads `__err`.
- The server suite's `validation::field_errors_round_trip_without_javascript`
  asserts that the `302` carries `__err` and that the page it lands on shows
  the error.
- `delivery::a_valid_submission_is_delivered_once_in_both_forms` asserts
  that a delivered submission's redirect has none.

## Other frameworks

The core crate does not depend on Axum.  With Actix Web or another backend,
provide the same context values through that framework's Leptos
integration.  The types are identical; check how that integration registers
server functions, since the one-closure rule above is a property of
`leptos_axum`.
