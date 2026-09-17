# RFC 016 — A true MSRV, pinned CI actions, and constant-time comparison from the crypto crates

**Status.** Accepted — 2026-09-17, with the owner's decisions below.  Milestone
M6 → 0.8.0 (P-24).
**Tracks.** Roadmap P-24.  Requirements NFR-COMPAT-02, NFR-SEC-* (token
verification), NFR-TEST-*.
**Handoffs.** [`../handoffs/016-msrv-and-ci-hygiene/README.md`](../handoffs/016-msrv-and-ci-hygiene/README.md)
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
- **A new CI job, `msrv`,** on toolchain 1.88, with three `--locked`
  checks:
  - native, `--all-features`;
  - wasm32 with `--features hydrate`;
  - wasm32 with the Workers feature set.
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
- **Updates:** Dependabot, for GitHub Actions only, **monthly**, with all
  actions grouped into **one** pull request (`.github/dependabot.yml`).  See
  the owner decisions.

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

## Owner decisions (2026-09-17)

Accepted.  The owner asked for the balance of profit and cost to be weighed
carefully; the architect's weighing follows, and each item was kept or cut on
it.

| Item | Profit | Cost | Kept as |
|------|--------|------|---------|
| **MSRV 1.88** and its docs | The published claim becomes true: today a 1.85 user fails to build | a one-line manifest change and four doc lines; a minor-release note, which 0.8.0 is anyway | kept |
| **The `msrv` CI job** | Every future raise becomes a visible decision instead of a silent false claim.  Each of the three builds has dependencies the others lack (the browser's `web-sys` features, the server's `reqwest`/`lettre`), so each can break the MSRV on its own | CI time only: `cargo check`, not a build or a test run.  **Measured on the architect's machine, 2026-09-17, with 1.88:** native `--all-features` 16 s, wasm32 `hydrate` 20 s, wasm32 Workers 6 s (dependencies already downloaded).  Public-repository minutes are free, and the job runs in parallel with the others, so it delays none | kept, all three checks: about a minute of CI for the one promise nothing else tests.  Tests, clippy and fmt stay off the MSRV job |
| **Actions pinned by SHA** | A retagged action cannot run new code with the repository token.  This is not hypothetical: in March 2025 the `tj-actions/changed-files` tags were repointed to malicious code | three lines, less readable; each needs an update path | kept, with the tag in a comment |
| **Keeping pins current** | Security and runtime fixes in the actions reach CI | Weekly Dependabot would open many pull requests, since `taiki-e/install-action` releases often.  By hand at each release costs attention at the busiest moment, and lapses if releases slow | **Dependabot, GitHub Actions only, monthly, grouped into one pull request**: at most about 12 small pull requests a year, each a SHA diff that CI verifies.  Cargo dependencies stay out: their updates remain deliberate |
| **`verify_slice` and `subtle`** | Removes our own constant-time code from the token path, in favour of the crypto crates' audited comparison | it touches a security-critical path, so it needs careful review and a break check; `subtle` becomes a direct dependency, but it is already built | kept: small, and the review cost is paid once |
| **Caching in CI** (not proposed) | faster runs | another third-party action to pin and update | not added |
