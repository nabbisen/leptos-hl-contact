# Troubleshooting

## The form submits but no email arrives

**Check 1 — Is `NoopDelivery` still in use?**

If you see log lines like `NoopDelivery: discarding contact form submission`,
you have not switched to `LettreSmtpDelivery`.

**Check 2 — Is `ContactDeliveryContext` provided to both sites?**

```
[ERROR leptos_hl_contact::server] ContactDeliveryContext not provided
```

You must call `provide_context` (or `delivery_context_fn`) in **both**:

1. The `handle_server_fns_with_context` closure
2. The `leptos_routes_with_context` closure

If either is missing, server-function calls will log this error and return
"Contact form is not configured."

**Check 3 — SMTP credentials**

Enable `RUST_LOG=debug` and look for `SMTP delivery failed` entries.  Common
causes:

| Log message | Likely cause |
|-------------|-------------|
| `Connection refused` | Wrong host or port |
| `authentication failed` | Wrong username / password |
| `TLS handshake error` | Wrong `SmtpTlsMode` (try switching StartTls ↔ Tls) |
| `invalid address` | Malformed `from_address` or `to_address` |

**Check 4 — Firewall / outbound rules**

Some hosting providers block outbound SMTP (port 587/465).  Verify your
server can reach the SMTP relay.

---

## "Contact form is not configured."

The `ContactDeliveryContext` is missing from the Leptos context in the server
function.  See [Axum Integration](./axum-integration.md) for the correct
wiring pattern.

---

## Validation errors appear for every field

This usually means the server is receiving empty form data.  Check that:

- Your `<ActionForm/>` fields use the exact argument names of `submit_contact`:
  `name`, `email`, `subject`, `message`, `website`.
- You are not accidentally nesting forms.

---

## The honeypot field is visible to users

The honeypot container is hidden via inline `position:absolute; left:-9999px`.
If your CSS resets or a utility class overrides `position` on all elements,
the field may become visible.  Check your global styles and ensure nothing
removes `position:absolute` from the honeypot wrapper.

---

## `cargo check` reports mismatched axum versions

`leptos_axum` v0.8 depends on `axum` **0.8**.  If you have `axum = "0.7"` in
your `Cargo.toml`, update it:

```toml
axum = { version = "0.8" }
```

And use the `async move` closure pattern for the server function handler (axum
0.8 requires closures to be explicitly async):

```rust,ignore
post({
    let ctx = ctx.clone();
    move |req: Request<Body>| {
        let ctx = ctx.clone();
        async move { handle_server_fns_with_context(ctx, req).await }
    }
})
```

---

## `edition 2024` parse error from older cargo

`leptos-hl-contact` requires Rust **1.85 or later** (the first stable release
to support the 2024 edition).  Run `rustup update` or install `rustc-1.91`
from your package manager.

---

## Tests fail with "no runtime"

Some async tests require the `tokio::test` macro.  Make sure your
`[dev-dependencies]` include:

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

---

## The `smtp-lettre` feature fails to compile on musl targets

`lettre`'s `tokio1-native-tls` feature depends on OpenSSL, which requires a
native TLS library.  On musl (Alpine, etc.) you may need to use
`tokio1-rustls-tls` instead.  File an issue if you need built-in rustls support
in `LettreSmtpDelivery`.

---

## Still stuck?

1. Enable full tracing: `RUST_LOG=leptos_hl_contact=trace,leptos=debug`
2. Check the [FAQ](./faq.md)
3. Open an issue on [GitHub](https://github.com/nabbisen/leptos-hl-contact/issues)
   with your Cargo.toml, the relevant log output, and a minimal reproduction.
