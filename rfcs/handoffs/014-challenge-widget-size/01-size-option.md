# Handoff 014-01 — `ChallengeSize` and `with_size`, on both render paths

**RFC.** [RFC 014](../../accepted/014-challenge-widget-size.md) D1–D4
**Roadmap.** P-44
**Requirements.** FR-ABUSE-10, FR-UI-03, NFR-COMPAT-05

## Goal

A site can ask for the vendor's compact widget, or Turnstile's flexible one.
- **On both render paths:** the server render, and the explicit render after
  client-side navigation.
- **The default markup** stays byte-identical to 0.7.

## Change scope

### 1. `src/config.rs`

- **`ChallengeSize`,** exactly as RFC 014 D1: `Normal` (the default),
  `Compact` and `Flexible`, with `ChallengeTheme`'s derives.  Rustdoc on each
  variant as in the RFC, including that `Flexible` has a 300 px minimum and
  applies to Turnstile only.
- **Re-export** it where `ChallengeTheme` is re-exported (`lib.rs`), and
  list it in the prelude if `ChallengeTheme` is there.
- **`ChallengeWidget`** gains a private `size: ChallengeSize` field.
  - `new` sets `Normal`.
  - `with_size` sits after `with_theme`, with a rustdoc example showing
    `Compact`.
- **A private mapping,** next to `turnstile_value`/`explicit_value`:
  `size_value(self, provider: &ChallengeProvider) -> Option<&'static str>`,
  per RFC D2.

### 2. `src/components.rs`

- **The server render:** the Turnstile, hCaptcha and reCAPTCHA v2 widget
  `<div>`s (currently around lines 275–300) get
  `data-size=widget.size_value()`.
  - It must be absent when `None`, as `data-theme` is for hCaptcha.
  - Follow how `explicit_value` is rendered there.
- **The explicit render,** `challenge_client::render_explicitly` (currently
  around lines 181–205): after the theme, set `size` when the mapping
  returns `Some`.  Do the same for Turnstile, hCaptcha and reCAPTCHA v2.
- **reCAPTCHA v3:** unchanged.

### 3. Tests

**Unit tests, `src/config/tests.rs`:**
- the full mapping table: 3 sizes × 4 providers;
- `ChallengeWidget::new(…)` has `Normal`;
- `with_size` sets it.

**Render tests, `src/components/tests.rs`,** next to the `data-theme` tests:
- for each visible provider: no `data-size` by default, and
  `data-size="compact"` with `Compact`;
- `Flexible`: `data-size="flexible"` for Turnstile, none for hCaptcha and
  reCAPTCHA v2;
- default markup: one test asserting the whole widget `<div>` for Turnstile
  equals 0.7's string.

**Browser test, `tests/browser/challenge.rs`:**
- **Extend `VendorStub`** (`tests/browser/support/vendor.rs`) to record the
  `params` object passed to `render`.  Today its closure ignores
  `_params`.
- **Add a test:** a Turnstile widget with `Compact`, mounted after the vendor
  global exists, is rendered with `size: "compact"`.
- **Add a second:** with the default, `params` has no `size` property.
- **Keep** the existing test's assertions unchanged.

**Break check, required.**  Remove the `size` line from `render_explicitly`.
- **Expected:** the compact browser test fails.
- **Report:** the output, then restore.

**Traceability:** add the new tests to the FR-ABUSE-10 row in `testing.md`.

### 4. Documentation (RFC D4)

- **`docs/src/security/challenge.md`:** a **Size** paragraph next to the
  theme option.
  - The values per provider, with Cloudflare's dimensions.
  - `Compact` for narrow layouts.
  - `Flexible`'s 300 px minimum, and that it is Turnstile only.
  - Cite the three vendor pages with "checked" dates.
- **`docs/src/reference/api.md`:** `ChallengeSize` and `with_size`.
- **`docs/src/development/external-design.md`:** the widget's attribute table,
  if it lists `data-theme`, gets `data-size`.

### 5. `CHANGELOG.md` `[Unreleased]`

**Added:** `ChallengeWidget::with_size` (`ChallengeSize::Compact`, and
`Flexible` for Turnstile), on the server render and after client-side
navigation.  The default markup is unchanged.

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The browser suite** (counts, including the two new tests) and the
  worker suite.
- **The `msrv` job** from RFC 016, if it has landed.
- **`mdbook build docs`:** no warnings.
- **CI** on the pushed commit.

## Review request

`.git-exclude/review-request/014-challenge-widget-size/01-size-option.md` must
include:
- the commit;
- the mapping table as implemented;
- the default-markup test;
- the break check output;
- the gates;
- CI.
