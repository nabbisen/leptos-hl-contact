# Handoff 02 — Browser tests and their CI job

**RFC.** [008](../../done/008-test-strategy.md), D4 and D6.
**Owner decision 2026-09-13:** the browser tests run in CI **on every push**.

## Purpose

Automate the hydrate-only behaviour that today exists only as CDP recordings.

## Change scope

- New `crates/leptos-hl-contact/tests/browser/` compiled for
  `wasm32-unknown-unknown`, gated so native `cargo test` skips it.
- `[target.'cfg(target_arch = "wasm32")'.dev-dependencies]`:
  `wasm-bindgen-test`, plus `web-sys` / `js-sys` features the tests need.
- `.github/workflows/ci.yml`: a new `browser` job.
- `docs/src/development/testing.md`: "Browser tests" section with the local
  command.

## Explicit non-change scope

No change to `src/`; no Node toolchain; no vendor script loaded from the
network (vendor globals are stubbed).

## Required implementation

1. **Harness.**  Mount `ContactForm` into a test document with
   `mount_to`.  Answer server-function requests by stubbing `window.fetch`
   through `js_sys`, returning scripted bodies and statuses.  Provide a
   controllable clock for the refresh logic, either by stubbing `Date.now`
   or by exposing nothing new in `src/` — if the clock cannot be controlled
   without a crate change, stop and report.
2. **Cases (RFC 008 D4).**
   - focus moves to the first invalid field after a field-error payload;
   - the hidden token value is unchanged after a failed submission;
   - `token_refresh_secs = None`: zero requests to `/api/form_token`;
   - `Some`: an empty field acquires exactly once; an overdue mounted token
     refreshes once; a fetched token schedules from arrival (advance the
     clock, assert the request count);
   - a widget element mounted after a stubbed vendor global exists triggers
     one `render` call on that global; a vendor `<script>` already in `<head>`
     is not inserted again.
3. **Versions.**  `wasm-bindgen-test-runner` must match the `wasm-bindgen`
   version the **workspace** lock resolves (0.2.121 on 2026-09-13; the example
   pins 0.2.126 separately, and the local runner is 0.2.126).  In CI install
   `wasm-bindgen-cli` at the lock's version, read from `Cargo.lock` rather
   than hard-coded, and say how.
4. **CI job `browser`.**  Same toolchain step as `check` with the wasm32
   target; install the matching CLI; run
   `cargo test -p leptos-hl-contact --target wasm32-unknown-unknown --features hydrate --test browser`
   (adjust to the layout you choose) with `CHROMEDRIVER` from the runner
   image.  Record the job's wall time from the run.

## Acceptance criteria

- Every D4 case passes locally and in the `browser` CI job.
- Deleting the arrival-based scheduling (local, uncommitted) makes the
  refresh test fail; paste it, restore.
- The native gates are unchanged and green.
- The job's added CI time reported.

## Prohibited shortcuts

`#[ignore]` on a browser test; `sleep`-based waits where the clock can be
advanced; loading real vendor scripts.

## Known risks

- `mount_to` in a test document and server-function client stubbing in Leptos
  0.8 are not yet demonstrated in this repository; report the first working
  pattern before writing all cases.
- Runner image Chromium and chromedriver versions must agree; pin or detect.

## Required evidence

The first working mount-and-stub test; case list with names; the deliberate
break; local and CI output; CI time added.
