//! # leptos-hl-contact
//!
//! A reusable, secure contact form plugin for [Leptos](https://leptos.dev) v0.8.
//!
//! ## Architecture
//!
//! The crate is structured as three cooperating layers:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  UI Component  (ContactForm)        │  client + server
//! ├─────────────────────────────────────┤
//! │  Server Function (submit_contact)   │  server only
//! ├─────────────────────────────────────┤
//! │  Delivery Backend (ContactDelivery) │  server only
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Feature flags
//!
//! | Flag            | Effect                                          |
//! |-----------------|--------------------------------------------------|
//! | `hydrate`       | Enables Leptos hydration for the client side.   |
//! | `ssr`           | Enables server-side rendering and server fns.   |
//! | `islands`       | Enables Leptos Islands architecture.            |
//! | `smtp-lettre`   | Enables the SMTP delivery adapter.             |
//! | `axum-helpers`  | Enables Axum-specific integration helpers.     |
//!
//! ## Quick start
//!
//! See the [`examples/axum-basic`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-basic)
//! directory for a complete working example.
//!
//! ## Security
//!
//! SMTP credentials and the recipient address live **only on the server**.
//! They are loaded from environment variables at startup and are never
//! serialised to WASM or returned to the client.
//!
//! See [`security`] for details.

pub mod config;
pub mod delivery;
pub mod error;
pub mod model;
pub mod security;

// UI components — compiled for both SSR and hydrate targets.
pub mod components;

// Server function — compiled only when `ssr` is active.
pub mod server;

// ---------------------------------------------------------------------------
// Re-exports for ergonomic imports
// ---------------------------------------------------------------------------

pub use components::ContactForm;
pub use config::{ContactFormClasses, ContactFormLabels, ContactFormOptions};
pub use delivery::{ContactDelivery, ContactDeliveryContext};
pub use error::{ContactDeliveryError, ContactValidationError};
pub use model::ContactInput;
pub use server::submit_contact;
