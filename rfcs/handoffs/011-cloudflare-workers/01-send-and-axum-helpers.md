# Handoff 011-01 — Spike, `Send` on a wasm32 server, `axum-helpers` without tokio, CI Workers check

**RFC.** [RFC 011](../../done/011-cloudflare-workers.md) D1, D2, D6
**Roadmap.** P-23
**Requirements.** NFR-PORT-02, NFR-PORT-01, FR-DEL-01, FR-DEL-07, FR-CFG-02
**Depends on.** RFC 012 handoff 01 merged, since both touch `testing.md` and
`CHANGELOG.md`.

## Goal

On a wasm32 server, an implementation of `ContactDelivery`,
`ChallengeVerifier` or `ContactFilter` may return a future that is not
`Send`.  `submit_contact` still compiles and stays `Send`.  `axum-helpers`
builds for wasm32.  CI checks it on every push.  Native builds are
unchanged.

## Step 0 — spike, reported before anything else

**Build a throwaway crate** in your scratch directory, not in the repository.
It depends on:
- this crate by path, with the D2 and D6 changes applied locally;
- `leptos_axum` with `default-features = false, features = ["wasm"]`;
- `axum` with `default-features = false`;
- `leptos` with `ssr`.

**It contains:**
- a `ContactDelivery` whose future holds an `std::rc::Rc` across an
  `.await`, so it is certainly not `Send`;
- a context closure providing it as `ContactDeliveryContext`;
- a router built with `leptos_routes_with_context` and a minimal app that
  renders `ContactForm`.

**Run:**

```
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
cargo check --target wasm32-unknown-unknown
```

**Report:**
- whether it compiles;
- the `Rc` line, which proves the future is not `Send`;
- the exact versions resolved.

**If it does not compile, stop.**  Report the error.  Do not work around it.
The design returns to review.

## Change scope

### 1. Future aliases (D2)

**`src/delivery.rs`:**
- **`DeliveryFuture<'a>`**, public, with a `Send` form for
  `not(all(target_arch = "wasm32", feature = "ssr"))` and a non-`Send` form
  otherwise.  RFC 011 D2 gives the code.
- **The trait.**  `ContactDelivery::deliver` returns `DeliveryFuture<'_>`.
  The trait keeps `Send + Sync + 'static`.
- **Rustdoc.**  The alias's rustdoc says why the two forms exist and that
  Leptos context still requires the implementing type to be `Send + Sync`.

**`src/challenge.rs`:** `VerifyFuture<'a>` in the same pattern, used by
`ChallengeVerifier::verify`.

**`src/filter.rs`:** `FilterFuture<'a>`, used by `ContactFilter::filter`.

**Every implementation inside the crate** switches to the alias:
- `NoopDelivery`, `LettreSmtpDelivery` and `DeliveryTimeout`;
- `HttpChallengeVerifier`;
- `FilterChain`;
- the doubles in `tests/server/support/doubles.rs`.

**Re-export** the three aliases at the crate root, next to their traits.

### 2. `SendWrapper` in `submit_contact` (D2)

**Dependency.**  In `crates/leptos-hl-contact/Cargo.toml`, add a
wasm32-only optional dependency:
`send_wrapper = { version = "0.6", features = ["futures"], optional = true }`.
`ssr` enables it with `"dep:send_wrapper"`; on native it is inert.

**`src/server.rs`.**  On a wasm32 server, await each extension future through
`send_wrapper::SendWrapper::new(…)`:
- `ctx.verifier.verify(&token)`;
- `filter.filter(&input)`, keeping the `.instrument(span)`: wrap the
  instrumented future;
- `delivery.deliver(input)`.

**Native stays a plain `.await`.**  A small private helper that is the
identity natively keeps each await site to one line.

**A code comment states the invariant:** a Worker isolate is
single-threaded, so the wrapper's thread check always passes, and a
multi-threaded wasm runtime would panic, not misbehave.

### 3. `axum-helpers` without tokio (D6)

**Normal dependencies:**
- `leptos_axum = { version = "0.8", optional = true, default-features = false }`
- `axum = { version = "0.8", optional = true, default-features = false }`

A comment says the application chooses: native Axum apps depend on
`leptos_axum` with its defaults, and a Worker enables its `wasm` feature.

**Native dev-dependencies:** `leptos_axum = "0.8"` and `axum = "0.8"`, both
with defaults, so the unit tests and the server suite still build natively.

**tokio.**  Move it from `[dependencies]` to a
`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` table, with the
same features, still optional.  `smtp-lettre` and `delivery-timeout` keep
naming it.

**A wasm32 build of `delivery-timeout` then has no timer until handoff 02.**
- `with_deadline` on a wasm32 server must still compile.  Until 02 lands,
  make it `compile_error!`-free by awaiting the delivery with no deadline.
- Mark it with `// TODO(011-02): JavaScript timer`.
- Handoff 02 removes that.  Say so in the review request.

### 4. CI (D1)

In the `check` job, after `check (wasm32, default features)`, add:

```yaml
      # Cloudflare Workers (RFC 011): the server path on wasm32, no tokio.
      # `challenge-http` joins this list in handoff 011-03.
      - name: check (wasm32 server, Workers features)
        env:
          RUSTFLAGS: --cfg getrandom_backend="wasm_js"
        run: >-
          cargo clippy -p leptos-hl-contact --target wasm32-unknown-unknown
          --no-default-features --features ssr,form-token,axum-helpers,delivery-timeout
          -- -D warnings
```

**Randomness (decided after step 0, 2026-09-15, RFC 011 D4 as amended).**
Add a `[target.'cfg(target_arch = "wasm32")'.dependencies]` table with
two optional dependencies, both enabled by `ssr`, and a comment naming
the cause (Leptos's `nonce` feature) and when to drop the 0.4 entry:

```toml
getrandom = { version = "0.3", features = ["wasm_js"], optional = true }
getrandom_04 = { package = "getrandom", version = "0.4", features = ["wasm_js"], optional = true }
```

The CI step must pass with only the `--cfg` flag set.  The decision and
its evidence are in `.git-exclude/reviewed/011-cloudflare-workers/01-step0-spike.md`.

## Tests

- **No behaviour changes natively.**  Every existing unit, server and
  browser test passes unchanged, apart from the doubles' return types.
- **A wasm32 compile test.**  Add
  `crates/leptos-hl-contact/tests/worker/main.rs`, gated
  `#![cfg(all(target_arch = "wasm32", feature = "ssr"))]`, with one module
  holding:
  - a delivery, a verifier and a filter whose futures hold an `Rc` across an
    `.await`;
  - a `#[wasm_bindgen_test]` that builds each as its context type and
    `.await`s it once.

  The CI step above compiles it with `--tests`.  Running it in the browser
  comes in handoff 02, which adds the job step.
- **Traceability.**  NFR-PORT-02 has no row yet, since it is not MUST
  until handoff 04.  Cite it in the test's doc comment.

## Acceptance

1. **Step 0** reported first; it compiled.
2. **Native gates** all pass: fmt, both clippy combinations, the wasm32
   default check, tests, doc.  So do both suites and both examples, with
   `--locked`.
3. **The new CI step** passes locally and in CI, including `--tests`.
4. **tokio is gone from wasm32** *(amended at review, 2026-09-15: `leptos_axum`
   depends on tokio unconditionally, so the criterion is that this crate adds
   no tokio dependency on wasm32 and `mio` is absent)*: the output of
   `cargo tree -p leptos-hl-contact --target wasm32-unknown-unknown --no-default-features --features ssr,form-token,axum-helpers,delivery-timeout -e normal -i tokio`
   (expected: nothing).
5. **Native `axum-helpers`, checked with an application that enables the
   defaults.**  Both examples already do; the `examples` job is green.
6. **Deliberate break.**  Remove the `SendWrapper` around the delivery
   await.  Show the wasm32 step fail with the `Send` error.  Then restore.
7. **CHANGELOG** `[Unreleased]`:
   - **Added:** the future aliases, and extension futures on a wasm32 server
     need not be `Send`.
   - **Changed:** `axum-helpers` no longer turns on `leptos_axum` and
     `axum` default features.
   - **Migration:** an application must depend on `leptos_axum` itself,
     which every Axum application using the form already does.

## Review request

`.git-exclude/review-request/011-cloudflare-workers/01-send-and-axum-helpers.md`
