# Feature Flags

| Flag | Implies | What it enables |
|------|---------|----------------|
| `hydrate` | — | Leptos client-side hydration |
| `ssr` | — | Leptos SSR + server functions |
| `islands` | — | Leptos Islands architecture |
| `smtp-lettre` | `ssr` | SMTP delivery via the `lettre` crate |
| `axum-helpers` | `ssr` | `delivery_context_fn` and `provide_contact_delivery` |

## Recommended combinations

| Use case | Features |
|----------|---------|
| Server binary | `ssr`, `smtp-lettre` |
| Server binary + Axum helpers | `ssr`, `smtp-lettre`, `axum-helpers` |
| WASM / client binary | `hydrate` |
| Tests | `smtp-lettre` (or none for model-only) |
| Islands app | `islands`, `smtp-lettre` |

## Default

`default = []` — The crate compiles with zero server or client dependencies.
This is intentional: downstream crates control exactly which features they
activate.
