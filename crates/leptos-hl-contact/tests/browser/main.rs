//! Browser tests (RFC 008 L3): `ContactForm` mounted in headless Chrome, with
//! server functions answered by a stubbed `fetch` and timers fired by the
//! test.  See `docs/src/development/testing.md`, "Browser tests".

#![cfg(all(target_arch = "wasm32", feature = "hydrate", not(feature = "ssr")))]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod support;

mod challenge;
mod focus;
mod token;
