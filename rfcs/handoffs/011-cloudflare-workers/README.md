# Handoffs — RFC 011, Cloudflare Workers as a supported server target

Companion to [RFC 011](../../accepted/011-cloudflare-workers.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release):
both suites in the gates, traceability kept true, CHANGELOG sections, and
deliberate breaks reported.
Review requests go to `.git-exclude/review-request/011-cloudflare-workers/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Spike, `Send` on a wasm32 server, `axum-helpers` without tokio, CI Workers check](./01-send-and-axum-helpers.md) | D1, D2, D6 | RFC 012 handoff 01 | large |
| 02 | [A JavaScript clock and timer](./02-clock-and-timer.md) | D4, D7 | 01 | medium |
| 03 | [Challenge verification over `fetch`; the visitor's IP](./03-fetch-verifier-and-client-ip.md) | D3, D5 | 02 | large |
| 04 | [Workers guide, CSP directives, records](./04-docs-and-records.md) | D8, D9 | 03 | medium |

**State.** Handoff 01 approved 2026-09-15 (`7bcf450`); step 0 decision recorded (option A).  Handoff 02 approved 2026-09-15 (`749e7e0`).  Handoff 03 approved 2026-09-15 (`3201c3a`, r2 `d348160`).  Handoff 04 approved 2026-09-15 (`91846d8`, r2 `2040522`: no `getrandom` flag,
behind `getrandom` 0.3.4).  **All handoffs approved.**  RFC 011 moves to `done/`
at the 0.7.0 release, after the reflerd.com runtime report.

**Strictly in order.**
- **01 first.**  Its step 0 spike can stop the design.
- **02 before 03.**  02 creates the private wasm timer that 03 reuses.
- **04 last.**  It documents what 01–03 built.

## Rules for this RFC

- **"wasm32 server"** means `all(target_arch = "wasm32", feature = "ssr")`.
  - Never use `target_arch = "wasm32"` alone for server code: the browser
    (`hydrate`) build must not change.
  - Never change native behaviour.  If a change seems to need it, stop and
    report.
- **Workers feature set:** `ssr,form-token,challenge-http,axum-helpers,delivery-timeout`.
- **The `getrandom` flag.**  Every wasm32 build that includes `form-token`
  needs `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`.  In CI, set it on
  that step only, so the other steps keep their build cache.
- **No new crate** beyond those RFC 011 names: `send_wrapper`, `js-sys`,
  `web-sys`, `wasm-bindgen`, `wasm-bindgen-futures`, `getrandom`.  All are
  already in the lock.  Report every `Cargo.lock` change.
- **Runtime behaviour on Workers** is verified by the reflerd.com team before
  release, not by these handoffs.  Do not claim Workers behaviour you did not
  run; say "compiles for wasm32" or "tested in headless Chrome".
