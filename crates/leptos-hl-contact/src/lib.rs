//! # leptos-hl-contact
//!
//! A reusable, secure contact form plugin for [Leptos](https://leptos.dev) v0.8.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │  UI Component  (ContactForm)             │  client + server
//! ├──────────────────────────────────────────┤
//! │  Server Function (submit_contact)        │  server only
//! ├──────────────────────────────────────────┤
//! │  Delivery Backend (ContactDelivery)      │  server only
//! └──────────────────────────────────────────┘
//! ```
//!
//! ## Feature flags
//!
//! | Flag           | Effect                                          |
//! |----------------|-------------------------------------------------|
//! | `hydrate`      | Enables Leptos hydration for the client side.   |
//! | `ssr`          | Enables server-side rendering and server fns.   |
//! | `islands`      | Enables Leptos Islands architecture.            |
//! | `smtp-lettre`  | Enables the SMTP delivery adapter.             |
//! | `axum-helpers` | Enables Axum-specific integration helpers.     |
//! | `form-token`   | Stateless HMAC-SHA256 form token; `submit_contact` requires `FormTokenContext` (fail-closed). |
//! | `csrf`         | Deprecated alias for `form-token`; removed in the next minor. |
//!
//! ## Quick start
//!
//! See
//! [`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
//! for complete production wiring, and
//! [`examples/axum-basic`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-basic)
//! for a local-development skeleton.
//!
//! ## Security
//!
//! SMTP credentials and the recipient address live **only on the server**.
//! They are never serialised to WASM or returned to the client.
//! See the [security documentation](https://github.com/nabbisen/leptos-hl-contact/blob/main/docs/src/security/README.md).

pub mod config;
pub mod delivery;
pub mod error;
pub mod model;
pub mod security;

// UI components — compiled for both SSR and hydrate targets.
pub mod components;

// Server function — compiled only when `ssr` is active.
pub mod server;

// Form token — compiled only when `form-token` is active.
#[cfg(feature = "form-token")]
pub mod form_token;

// Deprecated 0.4 aliases; removed in the next minor.
#[cfg(feature = "form-token")]
pub mod csrf;

// Challenge verification — trait, policy and decision table; no HTTP.
#[cfg(feature = "ssr")]
pub mod challenge;

// Axum integration helpers — compiled only when `axum-helpers` is active.
#[cfg(feature = "axum-helpers")]
pub mod axum_helpers;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use components::ContactForm;
pub use config::ContactErrorLabels;
pub use config::{
    ChallengeProvider, ChallengeTheme, ChallengeWidget, ContactFormClasses, ContactFormLabels,
    ContactFormOptions, ContactServerPolicy, ContactSuccessRedirect, InvalidChallengeConfig,
    InvalidRedirectPath, NoJsPolicy,
};
pub use delivery::{ContactDelivery, ContactDeliveryContext};
pub use error::{
    ContactDeliveryError, ContactErrorCode, ContactField, ContactFieldErrors,
    ContactValidationError, FieldError, FieldErrorCode,
};
pub use model::{ContactInput, MESSAGE_MAX_LEN};
pub use server::submit_contact;

#[cfg(feature = "ssr")]
pub use challenge::{
    ChallengeContext, ChallengeError, ChallengeOutcome, ChallengePolicy, ChallengeVerifier,
};

#[cfg(feature = "challenge-http")]
pub use challenge::http::HttpChallengeVerifier;

#[cfg(feature = "form-token")]
pub use form_token::{
    Binding, FormToken, FormTokenBinding, FormTokenConfig, FormTokenContext, FormTokenError,
    FormTokenIssuer, issue_form_token, issue_form_token_fn, issue_form_token_with_nonce,
    verify_form_token,
};
