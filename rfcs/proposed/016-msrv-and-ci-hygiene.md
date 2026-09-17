# RFC 016 — A true MSRV, pinned CI actions, and constant-time comparison from the crypto crates

**Status.** Proposed — 2026-09-17.  Milestone M6 → 0.8.0 (P-24, owner-approved
for M6 the same day).
**Tracks.** Roadmap P-24.  Requirements NFR-COMPAT-02, NFR-SEC-* (token
verification), NFR-TEST-*.
**Touches.** `Cargo.toml` (`rust-version`), `.github/workflows/ci.yml`,
`form_token.rs`, docs stating the MSRV, `testing.md`, `requirements.md`
(architect), `CHANGELOG.md`.

## Summary

- **The MSRV is false today.**  The crate says Rust 1.85, but the current
  lock does not build on it.  Set the MSRV to what builds (1.88), and check
  it in CI.
- **Pin every GitHub Action** by commit SHA.
- **Use the `hmac` crate's constant-time verification** instead of the
  hand-written comparison.

## Findings (architect, 2026-09-17)

**1. MSRV.**
- **The claim.**  `rust-version = "1.85"`, and requirement NFR-COMPAT-02
  ("MUST build on 1.85") is marked **Met**.  The introduction, quick start
  and troubleshooting pages say "Rust 1.85 or later".
- **What I ran:** `cargo +1.85 check -p leptos-hl-contact --all-features
  --locked` on `8b3809f`.
  - **Result:** refused, "rustc 1.85.1 is not supported".
  - **Named by cargo:** `leptos@0.8.19` and `either_of@0.1.9` require 1.88;
    several `icu_*` crates and `idna_adapter` require 1.86.
- **The same command on 1.88:** builds.
- **The traceability table** already says "nothing builds on 1.85 until
  P-24".
- **Who is affected.**  A user on 1.85 cannot build 0.7.0 with a fresh
  resolution.  The claim is wrong in practice, not only in CI.

**2. Actions.**  The workflow uses `actions/checkout@v6`,
`dtolnay/rust-toolchain@1.91` and `taiki-e/install-action@v2`, all by tag.
A moved tag runs new code with the repository's token.

**3. Token comparison.**
- **What the code does.**  `form_token.rs` compares the hex signature with a
  hand-written loop (`constant_time_eq`).
- **Its state.**  The loop is correct.  But the `hmac` crate, already a
  dependency, provides `Mac::verify_slice`, which compares in constant time
  through `subtle`.  `subtle` is already in the lock.
- **The trade.**  Using `verify_slice` removes our own crypto-adjacent code,
  and adds no crate.

## Design

### D1 — MSRV 1.88, stated and checked

- **The manifest:** `rust-version = "1.88"` in the workspace.
- **The claim, everywhere it appears:**
  - `introduction.md`, `quick-start.md`, `troubleshooting.md`: "Rust 1.88
    or later";
  - `testing.md`: the toolchain note and the NFR-COMPAT-02 traceability
    row.
- **A new CI job, `msrv`:** `dtolnay/rust-toolchain` pinned to 1.88.
  - `cargo check -p leptos-hl-contact --all-features --locked`;
  - `cargo check -p leptos-hl-contact --target wasm32-unknown-unknown
    --features hydrate --locked`;
  - `cargo check` of the Workers feature set on wasm32, `--locked`.
- **Not in the job:** tests, clippy and fmt stay on the main toolchain.
  Lints differ between versions, and the MSRV promise is about building.
- **The lock.**  The job uses the committed lock, so a dependency update
  that raises a transitive MSRV fails CI.  The dev team then reports it: the
  owner decides whether to raise the MSRV (a minor release) or hold the
  dependency back.
- **CHANGELOG, *Changed*:** "MSRV raised from 1.85 to 1.88; 1.85 could not
  build the dependency tree since Leptos 0.8.19".  The MSRV rule in
  NFR-COMPAT-02 requires it.

### D2 — Actions pinned by commit SHA

- **Every `uses:`** becomes `owner/action@<40-hex-sha> # <tag>`, the tag in a
  comment.
- **The SHAs** are resolved from the tag at handoff time.  The review
  request lists each tag → SHA, and how it was resolved, for example
  `gh api repos/<owner>/<repo>/git/ref/tags/<tag>`, following annotated
  tags.
- **`dtolnay/rust-toolchain`** is pinned by SHA too.  The toolchain version
  is then given with `with: toolchain: 1.91`, because the action's
  version-named branches cannot be SHA-pinned meaningfully.
- **Updates.**  See owner question 2.

### D3 — Token verification through `hmac`

- **The signature.**  Decode the submitted hex signature, then call
  `mac.verify_slice(&decoded)`.
  - **Wrong length or non-hex input** is `BadSignature`, as today.
  - **Timing:** decoding runs on attacker input of attacker-chosen length.
    That is not secret-dependent, so it leaks nothing.
- **The cookie binding**, `bound` compared with `nonce_hex`: use
  `subtle::ConstantTimeEq` on the bytes.
  - **The length check** stays first.  The nonce length is public.
  - **The dependency:** `subtle` becomes a direct dependency, at the
    version already in the lock, behind `form-token`.
- **The hand-written function is removed.**
- **The existing token tests** must pass unchanged.
- **One new unit test each:** a signature one hex digit off, and one of the
  wrong length.
- **Break check:** make `verify_slice`'s result ignored; the tampered-token
  tests must fail.

## Compatibility

- **MSRV:** raised to 1.88, which is a minor-release change, in 0.8.0.
- **Token verification:** no behaviour change.
- **CI:** no user impact.

## Handoffs (planned)

One handoff, D1–D3.  It is independent of RFCs 014 and 015, and can land
first.

## Owner questions (recommendations first)

1. **The MSRV: 1.88**, the lowest that builds today.  Alternative: follow
   Leptos's own `rust-version` on every Leptos update, stated as a policy.
   I recommend 1.88 now, with the CI job making every future raise a visible
   decision.
2. **Keeping pinned SHAs current: Dependabot for `github-actions` only**
   (`.github/dependabot.yml`), weekly.  Alternative: update by hand at each
   release, as a release-process step.  Dependabot keeps pins current
   without touching Cargo dependencies, whose updates stay deliberate.
