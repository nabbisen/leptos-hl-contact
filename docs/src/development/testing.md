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
| `challenge/tests.rs` | the challenge decision table, one test per row, with a mock verifier |
| `challenge/http/tests.rs` | `HttpChallengeVerifier` against a local responder: response shapes, the request, timeout and error mapping; live vendor tests (ignored) |
| `csrf/tests.rs` | token round-trip, tamper, wrong key, expiry, malformed input, constant-time compare |
| `delivery/noop/tests.rs` | async no-op call |
| `delivery/smtp/tests.rs` | message headers, `Reply-To` encoding, body content |
| `axum_helpers/tests.rs` | closure is `Clone` |

Tests are written from the [Requirements](./requirements.md) and
[External Design](./external-design.md), not from the code: when a test
and the specification disagree, fix one of them explicitly.

Integration tests that drive `submit_contact` end to end do not exist yet
(roadmap P-15).

## Live challenge tests

The `challenge-http` verifiers have tests that call the real vendor
endpoints with the vendors' published test keys.  They are `#[ignore]`d, so
neither `cargo test` nor CI runs them.  Run them by hand before a release
that touches `challenge/http.rs`:

```bash
cargo test -p leptos-hl-contact --no-default-features --features challenge-http --lib -- --ignored live_ --nocapture
```

They need outbound HTTPS to `challenges.cloudflare.com`, `api.hcaptcha.com`
and `www.google.com`.  Google's test secret accepts any token, so the
reCAPTCHA test shows the round trip, not a real check.

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
FORM_TOKEN_SECRET=$(openssl rand -hex 32) ALLOWED_ORIGIN=http://127.0.0.1:3000 cargo leptos watch
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

### Testing hydrated behaviour in a browser

Two rules, both learned the hard way:

**Rebuild the client bundle first, every time.** `cargo clean -p` does not
remove `target/site/pkg`, and a stale `.wasm` will happily serve code from
before your change — once making it look as though a fixed defect was still
present. Delete the outputs and rebuild, then check the timestamps:

```bash
cd examples/axum-with-security
rm -rf target/front target/site
cargo leptos build
ls -l target/site/pkg     # every file must be newer than your edit
```

**A build with narrower features can poison a later `--all-features` run.**
If `cargo test --all-features` reports a doctest failing with "found an item
that was configured out ... gated behind the `smtp-lettre` feature", the
crate's artefacts were last written by a build with a narrower feature set.
`cargo clean -p leptos-hl-contact` and re-run; CI never sees this because it
builds from scratch.  Do not read it as a defect until you have cleaned.

**Rebuild the example with `cargo leptos build`, not `cargo build`.** After
editing an example, a plain `cargo build` refreshes the binary but not the
generated bundle, and the two then disagree about the WASM filename: the
page asks for `<name>_bg.wasm`, gets a 404, and silently falls back to the
plain-POST path. The symptoms are a `TypeError: Failed to execute 'compile'
on 'WebAssembly': HTTP status code is not ok` in the console and a submit
recorded as a `Document` request where a `Fetch` was expected.

**Set `form.noValidate` before testing server-side validation.** The form
carries `required` and `type="email"`, so the browser blocks a submit with a
deliberately invalid value and the request never reaches the server — the
test then proves nothing:

```js
document.querySelector('form').noValidate = true;
```

## Building this book

```bash
cd docs && mdbook build
```
