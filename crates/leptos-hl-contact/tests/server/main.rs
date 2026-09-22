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
#[cfg(feature = "challenge-http")]
mod challenge_http;
mod delivery;
#[cfg(feature = "email-domain-check")]
mod email_domain;
mod filter;
mod form_token;
mod logging;
mod policy;
#[cfg(feature = "delivery-resend")]
mod resend;
mod routing;
mod silent;
mod site_fields;
mod validation;
