//! wasm32 server compile test (RFC 011 D2): on a Cloudflare Worker, an
//! extension future need not be `Send`.  Compiled by the CI step
//! `check (wasm32 server, Workers features)`; running it comes with handoff
//! 011-02.

#![cfg(all(target_arch = "wasm32", feature = "ssr"))]

mod not_send;
