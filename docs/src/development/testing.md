# Testing and Local Development

## Toolchain

Rust 1.85 or later (edition 2024).  CI runs on 1.91; `clippy` lint sets can
differ between versions, so run the gates on a recent stable before
pushing.

## The gates

These four commands are what CI runs.  All must pass on `main` at every
tag.

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```

`cargo test --all-features` runs the unit tests and the rustdoc examples.
Examples marked `ignore` or `no_run` are excluded on purpose (they need a
running server or environment variables).

## Test organisation

Tests live next to the module they cover, in `src/<module>/tests.rs`,
never inline.  Groups:

| Module | Covers |
|--------|--------|
| `model/tests.rs` | validation rules, trimming, honeypot detection, subject fallback |
| `config/tests.rs` | defaults |
| `error/tests.rs` | `ContactFieldErrors` JSON round-trip and sentinel parsing |
| `security/tests.rs` | header sanitisation |
| `server/tests.rs` | error-message shape (sentinel present or absent) |
| `csrf/tests.rs` | token round-trip, tamper, wrong key, expiry, malformed input, constant-time compare |
| `delivery/noop/tests.rs` | async no-op call |
| `delivery/smtp/tests.rs` | message headers, `Reply-To` encoding, body content |
| `axum_helpers/tests.rs` | closure is `Clone` |

Tests are written from the [Requirements](./requirements.md) and
[External Design](./external-design.md), not from the code: when a test
and the specification disagree, fix one of them explicitly.

Integration tests that drive `submit_contact` end to end do not exist yet
(roadmap P-15).

## Running the examples

`axum-basic` is server-only and needs nothing but Cargo:

```bash
cd examples/axum-basic && cargo run
```

`axum-with-security` ships a WASM client, so it is built and served with
[cargo-leptos](https://github.com/leptos-rs/cargo-leptos):

```bash
cargo install cargo-leptos
cd examples/axum-with-security
CSRF_SECRET=$(openssl rand -hex 32) ALLOWED_ORIGIN=http://127.0.0.1:3000 cargo leptos watch
```

Use `cargo leptos serve` for a one-shot build without the file watcher.
`ALLOWED_ORIGIN` must match the scheme, host and port in the address bar
exactly, or the origin check rejects every POST with `403`.

Without cargo-leptos the server binary still compiles and runs, but no
client bundle is produced, so the form falls back to the plain-POST path:

```bash
cd examples/axum-with-security && cargo check --features ssr
```

CI checks both targets of this example — `--features ssr` and
`--features hydrate --target wasm32-unknown-unknown --lib` — but does not run
cargo-leptos.

Both examples use `NoopDelivery`; switch to `LettreSmtpDelivery` and point it
at MailHog to see real messages
([Delivery Backends](../guides/delivery-backends.md#testing-delivery-locally)).

## Building this book

```bash
cd docs && mdbook build
```
