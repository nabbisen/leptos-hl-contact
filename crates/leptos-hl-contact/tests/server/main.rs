//! Server integration suite (RFC 008, layer L2).
//!
//! Every test drives the crate through a router built exactly as the
//! documentation tells integrators to: one context closure passed to
//! `leptos_routes_with_context`, a minimal app containing `ContactForm`, and
//! no hand-written server-function route.  Requests go through
//! `tower::ServiceExt::oneshot`, in process.
//!
//! The suite needs the server, the Axum helpers and the form token, which
//! `cargo test --all-features` enables.

#![cfg(all(
    feature = "ssr",
    feature = "axum-helpers",
    feature = "form-token",
    feature = "delivery-timeout"
))]

mod support;

mod binding;
mod challenge;
mod delivery;
mod filter;
mod form_token;
mod logging;
mod policy;
mod routing;
mod silent;
mod validation;
