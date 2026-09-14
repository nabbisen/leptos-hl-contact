//! The server path on wasm32 (Cloudflare Workers, RFC 011), run in headless
//! Chrome.
//!
//! - `not_send`: extension futures need not be `Send` (D2).
//! - `clock`: the form token reads the JavaScript clock (D4).
//! - `timer`: `DeliveryTimeout` enforces its deadline with a JavaScript
//!   timer (D7).
//!
//! ```bash
//! RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
//! cargo test -p leptos-hl-contact --target wasm32-unknown-unknown \
//!     --no-default-features --features ssr,form-token,delivery-timeout --test worker
//! ```

#![cfg(all(target_arch = "wasm32", feature = "ssr"))]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod not_send;

#[cfg(feature = "form-token")]
mod clock;

#[cfg(feature = "delivery-timeout")]
mod timer;
