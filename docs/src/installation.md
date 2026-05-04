# Installation

## Requirements

- Rust 1.85 or later
- A Leptos v0.8 application (SSR or Islands)

## Adding the dependency

### Server binary `Cargo.toml`

```toml
[dependencies]
leptos-hl-contact = { version = "0.2", features = ["ssr", "smtp-lettre"] }
```

To use the Axum convenience helpers:

```toml
[dependencies]
leptos-hl-contact = { version = "0.2", features = ["ssr", "smtp-lettre", "axum-helpers"] }
```

### Client (WASM) binary `Cargo.toml`

```toml
[dependencies]
leptos-hl-contact = { version = "0.2", features = ["hydrate"] }
```

## Feature flags

| Flag | What it enables |
|------|----------------|
| `hydrate` | Client-side hydration |
| `ssr` | Server-side rendering + server functions |
| `islands` | Leptos Islands architecture |
| `smtp-lettre` | SMTP delivery via `lettre` (implies `ssr`) |
| `axum-helpers` | `delivery_context_fn` and `provide_contact_delivery` helpers (implies `ssr`) |

## Environment variables (SMTP)

```bash
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=your-smtp-password
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
```

See [Security](./security.md) for credential management guidance.
