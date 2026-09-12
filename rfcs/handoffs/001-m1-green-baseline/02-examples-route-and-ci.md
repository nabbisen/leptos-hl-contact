# Handoff 02 — Examples start; CI checks them

**RFC.** [001](../../accepted/001-m1-green-baseline.md), decision D5.
**Roadmap.** P-09; P-05 (example version numbers).  **Requirements.** NFR-DOC-01, NFR-TEST-01.
**Depends on.** Handoff 01.

## Purpose

Make both examples run, and make CI prove they compile from now on.

## Background

`examples/axum-basic/src/main.rs` and
`examples/axum-with-security/src/main.rs` register
`.route("/api/*fn_name", …)`.  Axum 0.8 (`routing/path_router.rs`)
panics at router construction: "Path segments must not start with `*`.
For wildcard capture, use `{*wildcard}`".  Both examples therefore exit
before binding a port.  They are excluded from the workspace
(`Cargo.toml` `exclude`) so CI never compiled them.  Their manifests also
say `version = "0.3.2"` while the workspace is `0.3.3`.

## Change scope

- `examples/axum-basic/src/main.rs`, `examples/axum-with-security/src/main.rs`
- `examples/*/Cargo.toml` (version; Leptos metadata only if needed, see 3)
- `.github/workflows/ci.yml`
- `docs/src/development/testing.md` "Running the examples" only if the
  commands there turn out to be wrong.

## Explicit non-change scope

- Example behaviour, middleware, security settings, and the crate itself.
- Do not add the examples to the workspace `members` (that broke
  `cargo package`; see CHANGELOG 0.3.2).

## Required implementation

1. **Route syntax.**  `"/api/*fn_name"` → `"/api/{*fn_name}"` in both
   examples.
2. **Versions.**  Set both example manifests to `version = "0.3.3"`.
3. **Startup configuration.**  Run each example.  If
   `get_configuration(None)` returns an error because neither environment
   variables nor `[package.metadata.leptos]` are present, add to each
   example `Cargo.toml`:

   ```toml
   [package.metadata.leptos]
   output-name = "axum-basic"        # or "axum-with-security"
   site-addr   = "127.0.0.1:3000"
   ```

   and nothing else.  If startup already works, do not add it.  Report
   which case applied.
4. **Any other compile or startup failure** in the examples: fix it with
   the smallest change and list it in the review request under
   "differences".  If the fix would need a change in the crate, stop and
   report instead.
5. **CI job.**  Add to `.github/workflows/ci.yml` a job `examples` that
   installs the same toolchain as `check` (after handoff 01 this is
   `dtolnay/rust-toolchain@1.91`) and runs, for each example directory,
   `cargo check` with `working-directory` set to that directory.  Keep it a separate job so a slow example build does not
   delay the crate gates.  `check` is enough; do not run them.

## Required tests

None in Rust.  The CI job is the test.

## Required documentation updates

`docs/src/development/testing.md` "Running the examples" if commands
changed (for instance if a metadata table was added the env vars are no
longer needed).  `CHANGELOG.md` `[Unreleased]` → *Fixed* (examples start;
route syntax) and *Added* (examples CI job).  Nothing else; the book
already uses `{*fn_name}`.

## Acceptance criteria

- `cd examples/axum-basic && cargo run` logs the "Listening on" line and
  `curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:3000/`
  prints `200`, and `curl -s http://127.0.0.1:3000/ | grep -c 'id="contact-name"'`
  prints `1`.
- Same for `axum-with-security` with
  `CSRF_SECRET=$(openssl rand -hex 32) ALLOWED_ORIGIN=http://127.0.0.1:3000`,
  plus `curl -s http://127.0.0.1:3000/ | grep -c 'name="csrf_token" value="[0-9]'`
  prints `1` (a real token, not an empty value).
- `grep -rn '"/api/\*fn_name"' crates/ docs/ examples/ README.md` returns
  nothing.
- The `examples` CI job exists and passes on the pushed commit.
- Crate gates unchanged and green.

## Prohibited shortcuts

Adding the examples to the workspace; disabling the panic by catching it;
pinning a pre-0.8 axum in the examples.

## Module boundaries

Examples and CI only.

## Compatibility constraints

None for the crate.  Examples are `publish = false`.

## Security constraints

The `axum-with-security` example must keep refusing to start without
`CSRF_SECRET` and `ALLOWED_ORIGIN`.  Do not add defaults.

## Known risks

- The examples have not been compiled by CI since 0.3.2; other rot may
  surface.  Item 4 bounds what you may fix.
- `examples/axum-with-security` depends on `tower_governor` 0.8; if its
  API drifted, the smallest compiling change is in scope.
- SSR-only examples reference a client bundle that does not exist; the
  page still renders and a 404 for `/pkg/*` in the log is acceptable.

## Required evidence

Startup log excerpt and the curl outputs for both examples; CI job URL or
log tail; gate outputs.
