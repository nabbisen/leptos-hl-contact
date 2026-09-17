# RFC 014 — A size option for the challenge widget

**Status.** Proposed — 2026-09-17.  Milestone M6 → 0.8.0; the owner approved
the feature (P-44) the same day.
**Tracks.** Roadmap P-44.  Requirements FR-ABUSE-10, FR-UI-03, NFR-COMPAT-05.
**Touches.** `config.rs` (`ChallengeSize`, `ChallengeWidget::with_size`),
`components.rs` (both render paths), tests (unit, render, browser), docs
(Challenge, API reference), `CHANGELOG.md`.
**Origin.** The reflerd.com team, 2026-09-17: at Turnstile's normal 300 px
width, the widget widened pages on phones narrower than about 374 px.

## Summary

Add `ChallengeWidget::with_size(ChallengeSize)`, alongside `with_theme`.
- **The values:** `Normal`, the default; `Compact`; and `Flexible`, for
  Turnstile only.
- **The default markup** does not change.

## Vendor values (checked 2026-09-17)

| Provider | Attribute and `render` parameter | Values | Source |
|----------|----------------------------------|--------|--------|
| Turnstile | `data-size` / `size` | `normal` 300×65 px; `flexible` 100 % width, minimum 300 px, ×65; `compact` 150×140 px | Cloudflare, Turnstile "Widget configurations" |
| hCaptcha | `data-size` / `size` | `normal`, `compact`; also `invisible` | hCaptcha, "Configuration" |
| reCAPTCHA v2 | `data-size` / `size` | `compact`, `normal` (default); also `invisible` | Google, reCAPTCHA "Display" |
| reCAPTCHA v3 | — | no visible widget | — |

**`flexible` has a 300 px minimum.**  It does not help a layout narrower
than 300 px plus padding; only `compact` does.  The rustdoc and the
Challenge page say so.

## Design

### D1 — The type

```rust,ignore
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChallengeSize {
    /// The vendor's default.  No attribute is rendered.
    #[default]
    Normal,
    /// Turnstile 150×140 px; hCaptcha's and reCAPTCHA v2's compact widget.
    Compact,
    /// Turnstile only: full width, at least 300 px.  Other providers render
    /// their default size.
    Flexible,
}

impl ChallengeWidget {
    pub fn with_size(self, size: ChallengeSize) -> Self;
}
```

- **Naming and placement** mirror `ChallengeTheme` and `with_theme`.
- **The derives** are `ChallengeTheme`'s exactly.  Neither type is
  serialized: `ChallengeWidget` derives no `serde`.
- **The field on `ChallengeWidget` is private,** like `theme`, and set only
  through `with_size`.  So adding it breaks no caller.
- **`invisible` is not offered.**  It changes how a token is obtained: the
  site must call `execute`.  The no-JavaScript path and the gate assume a
  visible widget, so offering it would be a separate RFC.

### D2 — Rendering

- **A private mapping,** as for the theme, returns
  `Option<&'static str>` per provider:
  - `Normal` → `None`, for every provider;
  - `Compact` → `Some("compact")` for Turnstile, hCaptcha and reCAPTCHA v2;
  - `Flexible` → `Some("flexible")` for Turnstile, and `None` for the
    others;
  - reCAPTCHA v3 → `None`.
- **The server render** (`components.rs`, the widget `<div>`s): `data-size`
  is present only when the mapping returns `Some`.  With `Normal`, the
  markup is byte-identical to 0.7.
- **The explicit render after client-side navigation**
  (`challenge_client::render_explicitly`): set `size` in the parameters
  object under the same rule.
  - **Why this matters.**  Missing this path would render a compact widget
    on first load and a normal one after navigation.  That is the defect
    this RFC must not ship.

### D3 — Tests

- **Unit tests:** the mapping table, including `Flexible` → `None` for
  hCaptcha and reCAPTCHA v2, and that `ChallengeWidget::new` starts at
  `Normal`.
- **Render tests (SSR):**
  - `Normal` renders no `data-size` for each provider;
  - `Compact` renders `data-size="compact"` for the three visible providers;
  - `Flexible` renders it only for Turnstile.
- **Browser test (L3):** the explicit-render path passes `size` to the
  stubbed vendor `render` when set, and omits it with `Normal`.  Reuse the
  existing vendor stub.
- **Traceability:** FR-ABUSE-10.

### D4 — Documentation

- **Challenge page:** a "Size" paragraph next to the theme.
  - **The table** above.
  - **Advice for narrow layouts:** `Compact` is the choice for layouts
    under about 340 px of available width.
  - **The limit of `Flexible`:** its 300 px minimum.
- **API reference:** `ChallengeSize`, `with_size`.
- **CHANGELOG, *Added*:** the option, and that the default is unchanged.

## Compatibility

Additive.
- **Existing markup:** unchanged.
- **Existing callers:** unaffected, because the new field is private.

## Owner questions

None.  The recommended defaults above follow `with_theme`'s precedent.
