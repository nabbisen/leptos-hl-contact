# Handoff 016-01 — MSRV 1.88 and its CI job, actions pinned with Dependabot, token comparison through `hmac`

**RFC.** [RFC 016](../../done/016-msrv-and-ci-hygiene.md) D1–D3, with the owner decisions
**Roadmap.** P-24
**Requirements.** NFR-COMPAT-02, FR-SEC token verification (see `form_token.rs` rustdoc), NFR-TEST-*

## Goal

- **The MSRV is true and checked.**
- **Every action is pinned by commit SHA,** and kept current by one monthly
  pull request.
- **The form token's comparisons** use the crypto crates, not our own loop.

## Change scope

### 1. MSRV (D1)

- **`Cargo.toml` (workspace):** `rust-version = "1.88"`.
- **The claim in the docs**, "Rust 1.85 or later" → "Rust 1.88 or later":
  - `docs/src/introduction.md:51`;
  - `docs/src/getting-started/quick-start.md:10`;
  - `docs/src/help/troubleshooting.md:75`;
  - `docs/src/development/testing.md:5` (edition 2024 stays).
- **Traceability:** the NFR-COMPAT-02 row in `testing.md` (currently line 338)
  names the `msrv` CI job, instead of "nothing builds on 1.85 until P-24".
- **Grep:** `git grep -n "1\.85" -- README.md docs/src crates` then prints
  only history, if any.  Report every hit.
- **Do not edit `requirements.md`:** the architect updates NFR-COMPAT-02 at
  review.

### 2. The `msrv` CI job (D1)

In `.github/workflows/ci.yml`, add a job **`msrv`**.
- **Toolchain:** 1.88, with the `wasm32-unknown-unknown` target.
- **Checks,** each `--locked`:
  - `cargo check -p leptos-hl-contact --all-features --locked`;
  - `cargo check -p leptos-hl-contact --target wasm32-unknown-unknown --features hydrate --locked`;
  - `cargo check -p leptos-hl-contact --target wasm32-unknown-unknown --no-default-features --features ssr,form-token,challenge-http,axum-helpers,delivery-timeout --locked`.
- **Comment:** one line saying why the job exists (NFR-COMPAT-02, RFC 016),
  and that a failure means a dependency raised the MSRV.  That is an owner
  decision, not something to work around.
- **Not in this job:** clippy, fmt or tests.  **No caching action.**

### 3. Actions pinned by SHA (D2)

- **Every `uses:`** in `ci.yml` becomes `owner/action@<40-hex-sha> # <tag>`:
  `actions/checkout`, `dtolnay/rust-toolchain`, `taiki-e/install-action`,
  and the new job's.
- **Resolve each SHA from the tag the workflow uses today,** with `gh api`,
  and follow annotated tags to the commit.  Report tag → SHA and the
  commands in the request.
- **`dtolnay/rust-toolchain`:** it selects the toolchain by its tag name
  today (`@1.91`).  Pin its SHA, and pass the version as `with: toolchain:
  1.91` (and `1.88` in the new job).  Check in the action's README that
  `toolchain` is the input name, and cite it.
- **Behaviour must not change:** CI runs the same toolchains and steps as
  before.

### 4. Dependabot, actions only (owner decision)

New `.github/dependabot.yml`:
- **`package-ecosystem: github-actions`, `directory: /`.**
- **Schedule:** `interval: monthly`.
- **One group** matching all actions (`patterns: ["*"]`), so one pull
  request a month at most.
- **No `cargo` ecosystem.**  Cargo updates stay deliberate.

### 5. Token comparison (D3), `crates/leptos-hl-contact/src/form_token.rs`

**Step A: confirm the API first.**
- **What to check:** that `hmac` 0.13 (with `digest` 0.11) provides
  `Mac::verify_slice`, and how its error type reads.  Cite the source path
  in the registry.
- **If it does not exist, stop and report.**

**The signature** (currently line 298–302).
- Build the MAC over the payload.
- `hex::decode` the submitted signature:
  - a decode failure → `BadSignature`;
  - otherwise `verify_slice(&decoded)`, where `Err` → `BadSignature`.
- **Leave the signing side unchanged:** `sign` still produces hex.  Only
  verification changes.

**The binding** (currently line 305–310).
- Compare `bound.as_bytes()` with `nonce_hex.as_bytes()` through
  `subtle::ConstantTimeEq`.
- A length difference returns `BindingMismatch` before comparing, as today.

**Remove** `constant_time_eq`.

**`Cargo.toml` (crate):** `subtle = { version = "2.6", optional = true }`,
added to the `form-token` feature list.
- **The lock:** it must record no new package, since `subtle` 2.6.1 is
  already there.
- **Report** the `Cargo.lock` diff; expect only a new dependency edge.

**Rustdoc:** "The signature comparison is constant-time" (line 264) stays
true.  Name `hmac`'s `verify_slice` in it.

### 6. Tests, `src/form_token/tests.rs`

**Existing tests pass unchanged,** notably:
- `a_tampered_signature_is_a_bad_signature`;
- the binding-mismatch test.

**Add:**
- a signature of the right length, one hex digit changed → `BadSignature`;
- a signature one byte (two hex digits) short → `BadSignature`;
- a signature that is not hex → `BadSignature`.

**Break check, required.**  Temporarily ignore `verify_slice`'s result, so
verification always passes.
- **Expected:** the tampered-signature tests fail.
- **Report:** the failing output, then restore.

### 7. `CHANGELOG.md` `[Unreleased]`

- **Changed:**
  - "MSRV raised from 1.85 to 1.88.  1.85 could not build the dependency
    tree since Leptos 0.8.19."
  - "Form token signatures are verified with `hmac`'s constant-time
    `verify_slice`; behaviour is unchanged."
- **Migration:** "Rust 1.88 or later is required."

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The three MSRV checks** run locally on 1.88, with exit codes.
- **The worker and browser suites** (form-token code changed): counts.
- **CI:** the run on the pushed commit, all jobs including `msrv`, with the
  job list.
- **`mdbook build docs`:** no warnings.

## Review request

`.git-exclude/review-request/016-msrv-and-ci-hygiene/01-msrv-actions-and-token-compare.md`
must include:
- the commit;
- the tag → SHA table with its commands;
- the `verify_slice` API citation;
- the lock diff;
- the break check;
- the gates;
- the CI run.
