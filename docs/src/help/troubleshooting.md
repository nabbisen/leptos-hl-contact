# Troubleshooting

## Server panics at startup: "Path segments must not start with `*`"

Axum 0.8 changed the wildcard syntax, but you do not need this route at all:
`leptos_routes_with_context` registers every server function itself.  Delete
the `.route("/api/…", …)` line and provide your context in the closure you
pass to `leptos_routes_with_context`.  See
[Axum Integration](../guides/axum-integration.md).

Releases up to and including 0.3.4 told integrators to write that route;
before 0.3.4 the documentation spelled it with the Axum 0.7 form
`"/api/*fn_name"`, which is what panics.  A value provided only in that
route's closure never reached the server function anyway.

## "This form is not available right now."

`ContactDeliveryContext` is missing from the context closure.  Provide it
(or `delivery_context_fn`) in the closure you pass to
`leptos_routes_with_context`; that one closure serves page renders and
server functions alike.  The log line is:

```text
ERROR leptos_hl_contact::server: ContactDeliveryContext not provided — check server setup
```

## The banner says the form is unavailable, with the `csrf` feature on

The `csrf` feature is enabled but `CsrfConfigContext` is missing from the
context closure.

## "Your session token expired. Please reload the page and try again."

One of:

- `CsrfToken` is not provided in the context closure, so the hidden field is
  empty.  Add `provide_context(generate_csrf_token(&csrf))` there.
- The token is older than `token_ttl_secs` (default one hour).
- The secret changed since the page was rendered: a restart with a new
  `CSRF_SECRET`, or instances behind a load balancer that disagree.

## The form submits but no email arrives

1. Still using `NoopDelivery`?  Look for
   `NoopDelivery: discarding contact form submission` at `debug`.
2. Check the `SMTP delivery failed` log line:

   | Message | Likely cause |
   |---------|--------------|
   | `Connection refused` | wrong host or port |
   | `authentication failed` | wrong username or password |
   | `TLS handshake` | wrong `SmtpTlsMode`; try `StartTls` ↔ `Tls` |
   | `invalid address` | malformed `from_address` or `to_address` |

3. Some hosts block outbound 587/465.  Verify the relay is reachable.

## Every field shows an error

The server received empty fields.  Make sure the form is the crate's
`ContactForm` (field names `name`, `email`, `subject`, `message`,
`website`, `csrf_token`) and that it is not nested inside another `<form>`.

## The honeypot field is visible

Some global CSS overrides the inline `position:absolute` on its wrapper.
Exempt the wrapper or drop the reset rule.

## `cargo check` complains about axum versions

`leptos_axum` 0.8 needs `axum = "0.8"`.  Update the dependency and use the
`async move` closure form shown in the [Quick Start](../getting-started/quick-start.md).

## `edition 2024` parse error

Rust 1.85 or later is required.  `rustup update`.

## `smtp-lettre` fails to build on musl

`lettre`'s `tokio1-native-tls` needs OpenSSL.  On Alpine and similar, build
with a native TLS library available, or open an issue if you need a rustls
option in `LettreSmtpDelivery`.

## Known issues

Confirmed by the maintainers; scheduled in `ROADMAP.md`.

| Symptom | Cause | Item |
|---------|-------|------|
| Without JavaScript, a successful submit shows no confirmation | The redirect back to the page carries no success signal | P-13 |

## Still stuck?

1. `RUST_LOG=leptos_hl_contact=trace,leptos=debug`
2. Read the [FAQ](./faq.md)
3. Open an [issue](https://github.com/nabbisen/leptos-hl-contact/issues)
   with your `Cargo.toml`, the relevant log lines, and a minimal reproduction.
