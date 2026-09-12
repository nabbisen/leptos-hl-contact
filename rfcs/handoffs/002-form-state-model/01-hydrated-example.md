# Handoff 01 — Hydrated example

**RFC.** [002](../../done/002-form-state-model.md), design D4.
**Requirements.** NFR-DOC-01, NFR-TEST-01.

## Purpose

Give the repository one example that ships a WASM client, so hydration
behaviour can be demonstrated and reviewed.

## Background

Both examples are server-only binaries.  `ContactForm` runs there in SSR
mode only; the P-10/P-11 defects and their fix exist only under
hydration.  `cargo-leptos` is the standard tool for a Leptos project with
a server binary and a hydrate library.

## Change scope

`examples/axum-with-security/` only (`Cargo.toml`, `src/`), the
`examples` job in `.github/workflows/ci.yml`, and
`docs/src/development/testing.md` "Running the examples".

## Explicit non-change scope

The crate; `examples/axum-basic` stays server-only apart from a minimal
`[package.metadata.leptos]` table (`output-name`, `site-addr`) that
silences the `LEPTOS_OUTPUT_NAME` startup notice both examples print today; the example's security
layers (body limit, rate limit, origin check, token) stay exactly as they
are.

## Required implementation

1. **Manifest.**  Convert to the cargo-leptos layout:
   - `[lib] crate-type = ["cdylib", "rlib"]`, `[[bin]] name = "axum-with-security"`.
   - Features: `hydrate = ["leptos/hydrate", "leptos-hl-contact/hydrate"]`;
     `ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr", "dep:leptos_axum", "dep:axum", "dep:tokio", "dep:tower", "dep:tower-http", "dep:tower_governor", "dep:tracing-subscriber", "dep:dotenvy", "dep:url", "leptos-hl-contact/ssr", "leptos-hl-contact/smtp-lettre", "leptos-hl-contact/axum-helpers", "leptos-hl-contact/csrf"]`.
   - `leptos-hl-contact = { path = "../../crates/leptos-hl-contact" }` with **no**
     default features; everything comes through the two example features.
   - Add `leptos_meta = "0.8"`, `console_error_panic_hook = "0.1"`,
     `wasm-bindgen = "=<the version cargo-leptos on your machine requires>"`.
   - `[package.metadata.leptos]`: `output-name = "axum-with-security"`,
     `site-root = "target/site"`, `site-pkg-dir = "pkg"`,
     `site-addr = "127.0.0.1:3000"`, `reload-port = 3001`,
     `bin-features = ["ssr"]`, `lib-features = ["hydrate"]`,
     `lib-profile-release = "wasm-release"`, plus the
     `[profile.wasm-release]` block from the cargo-leptos template.
2. **`src/lib.rs`.**  `pub mod app;` and the standard
   `#[cfg(feature = "hydrate")] #[wasm_bindgen::prelude::wasm_bindgen] pub fn hydrate()`
   that installs the panic hook and calls `leptos::mount::hydrate_body(App)`.
3. **`src/app.rs`.**  Add `pub fn shell(options: LeptosOptions) -> impl IntoView`
   rendering `<!DOCTYPE html><html lang="en"><head><meta charset/>
   <meta viewport/><AutoReload options/><HydrationScripts options/>
   <MetaTags/></head><body><App/></body></html>`; `App` calls
   `provide_meta_context()`.  Keep the page content as it is.
4. **`src/main.rs`.**  Wrap in `#[cfg(feature = "ssr")]`; add the
   `#[cfg(not(feature = "ssr"))] fn main() {}` stub.  Use
   `leptos_routes_with_context(&leptos_options, routes, <ctx closure>,
   { let o = leptos_options.clone(); move || shell(o.clone()) })` and add
   `.fallback(leptos_axum::file_and_error_handler(shell))` so `/pkg/*` is
   served.  Nothing else in the router or middleware changes.
5. **CI.**  In the `examples` job, for this example run
   `cargo check --features ssr` and
   `cargo check --features hydrate --target wasm32-unknown-unknown --lib`.
   The Debian toolchain may lack the wasm32 target; if
   `apt-get install libstd-rust-dev-wasm32` (or the 1.91-suffixed package)
   does not work, switch **this job only** to `dtolnay/rust-toolchain@1.91`
   with `targets: wasm32-unknown-unknown`.  Do not run `cargo leptos` in CI.
6. **Docs.**  `testing.md` "Running the examples": the security example is
   now started with `cargo leptos watch` (or `cargo leptos serve`) with the
   same two environment variables; state that `cargo install cargo-leptos`
   is required.

## Required tests

None in Rust.  CI checks both targets.

## Acceptance criteria

- `cargo leptos build` succeeds; `cargo leptos serve` with
  `CSRF_SECRET` and `ALLOWED_ORIGIN` set serves `/`, and the page loads
  `/pkg/axum-with-security.js` and `.wasm` with HTTP 200 (browser network
  panel or `curl -I`).
- In the browser console there are no hydration errors or panics.
- Submitting the form with JavaScript enabled does **not** cause a full
  page load (network panel shows a `fetch` to `/api/submit_contact`).
  Record this; it proves hydration is live.  The known P-10/P-11
  behaviour will still be visible until handoff 02 lands; that is
  expected here.
- `examples` CI job passes for both targets.
- `cargo check --features ssr` still works without cargo-leptos.

## Prohibited shortcuts

Committing `target/site` or built artifacts; disabling the origin check
or the token to make the WASM path work; enabling `leptos-hl-contact/ssr`
in the hydrate feature.

## Compatibility and security constraints

Example only.  `CSRF_SECRET` and `ALLOWED_ORIGIN` remain mandatory.

## Known risks

- `wasm-bindgen` version pinning between cargo-leptos and the lock file;
  pin to what `cargo leptos build` asks for and note the version.
- The `csrf` feature must not be enabled in the WASM build (it implies
  `ssr`).  The component handles the absent feature by rendering the
  hidden field from an empty string; the SSR value survives hydration.
- The origin check reads `Origin`; browsers send it on `fetch` POSTs.  If
  it fails in development, `ALLOWED_ORIGIN` must match the scheme, host
  and port shown in the address bar exactly.

## Required evidence

`cargo leptos build` tail; startup log; network panel screenshot or
`curl -I` of the `.wasm`; console screenshot; the CI job log tail.
