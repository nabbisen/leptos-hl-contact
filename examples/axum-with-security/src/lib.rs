// examples/axum-with-security/src/lib.rs
//
// Shared between the server binary (`ssr`) and the WASM client (`hydrate`).
// The binary renders `app::App` on the server; the WASM module hydrates the
// same tree in the browser.

pub mod app;

/// Entry point of the WASM client.
///
/// `cargo-leptos` names the generated module after `output-name`, and the
/// `HydrationScripts` component in [`app::shell`] loads it.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
