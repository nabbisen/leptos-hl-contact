# External Design

> **Document status.** Draft 1, proposed by the architect on 2026-09-12
> against baseline `0.3.3` (commit `8d29d5a`).  Awaiting owner approval.
>
> **What this document is.** The external (basic) design: everything an
> integrator, a visitor, an operator, or an adversary can observe at the
> crate's boundaries.  Interfaces, behaviour, data formats, guarantees.
> It does not describe module internals; see [Architecture](./architecture.md).
> Exhaustive signatures are in the [API Reference](../reference/api.md).
>
> **Conventions.** Requirement identifiers in brackets, for example
> [FR-SUB-02], refer to the [Requirements Specification](./requirements.md).
> Where the current implementation differs from the design, the row is
> marked **current** / **target** and the roadmap item is cited.

---

## 1. System context

```text
                                  integrator-operated                     third parties
 ┌──────────────┐        ┌────────────────────────────────┐
 │   Visitor    │ HTTPS  │  Reverse proxy / TLS / WAF      │
 │  browser,    ├───────▶│  (sets X-Forwarded-For)         │
 │  JS or no-JS │        └───────────────┬────────────────┘
 └──────────────┘                        │ HTTP
        ▲                ┌───────────────▼────────────────┐
        │                │  Leptos + Axum application      │
        │  optional      │  ┌────────────────────────────┐ │
        │  Turnstile     │  │  leptos-hl-contact          │ │      ┌────────────────┐
        │  widget        │  │  ContactForm   (SSR + CSR)  │ │      │ SMTP relay or  │
        │                │  │  submit_contact (SSR only)  │ │      │ mail API       │
        │                │  │  ContactDelivery ───────────┼─┼─────▶│ → operator     │
        │                │  └────────────────────────────┘ │      │   inbox        │
        │                │  application provides:          │      └────────────────┘
        │                │   · Leptos contexts (2 sites)   │
        │                │   · rate limit, origin check,   │      ┌────────────────┐
        │                │   · body limit, TLS             ├─────▶│ Turnstile      │
        │                └────────────────────────────────┘      │ siteverify     │
        │                                                        └────────────────┘
        └──── challenge script (optional) ───────────────────────────────┘
```

### 1.1 Trust boundaries

| Zone | Trust | Notes |
|------|-------|-------|
| Browser | Untrusted | Every value it sends is validated again on the server [FR-SUB-05] |
| Application process | Trusted | Holds SMTP credentials, token secret, recipient address |
| Reverse proxy | Trusted for client IP | Rate limiting keys on `X-Forwarded-For` only if the proxy sets it |
| SMTP relay / mail API | Semi-trusted | Receives credentials and message content over TLS |
| Turnstile (optional) | Third party | Receives visitor signals; must be disclosed by the integrator [NFR-PRIV-02] |

### 1.2 Responsibility split

| Concern | Crate | Application |
|---------|-------|-------------|
| Form markup, states, ARIA | ✅ | styles only |
| Server-side validation, honeypot, policy | ✅ | — |
| Form token issue and verify (`form-token`) | ✅ | provide secret and the context |
| Origin / Referer validation | documents + example | ✅ middleware |
| Rate limiting | documents + example | ✅ middleware |
| Request body limit | documents + example | ✅ layer |
| TLS | — | ✅ proxy |
| Delivery transport | ✅ SMTP, no-op; trait for others | choose and configure |
| Secrets | typed fields, redacted `Debug` | load from env or secret store |
| Challenge (Turnstile, hCaptcha, reCAPTCHA) | ✅ opt-in `challenge` feature (RFC 005) | provide keys; disclose the vendor |

---

## 2. Integration model

### 2.1 Steps an integrator performs

1. Add the crate to the server binary with `ssr` plus the backends and
   helpers wanted, and to the WASM binary with `hydrate`.
2. Construct a delivery backend and wrap it as `Arc<dyn ContactDelivery>`.
3. (Optional) Construct `FormTokenConfig` from a secret; construct
   `ContactServerPolicy`.
4. Provide the context values in the context closure (§2.2).
5. Add application-level layers: body limit, rate limit, origin check.
6. Place `<ContactForm/>` in a page, passing classes, labels, options.

### 2.2 The context closure and required values

All values are provided in the single closure passed to
`leptos_routes_with_context`.

| Context value | Type | Provided in the context closure | If missing |
|---------------|------|---------------------------------|------------|
| Delivery backend | `ContactDeliveryContext` = `Arc<dyn ContactDelivery>` | **required** | `error` log; generic "not configured" message [FR-SUB-08] |
| Token config (`form-token`) | `FormTokenContext` = `Arc<FormTokenConfig>` | **required** | fail-closed: `error` log; generic "security not configured" message [FR-ABUSE-04] |
| Per-request token (`form-token`) | `FormToken` | **required**, freshly generated per request; read only by page renders | form renders an empty hidden field; every submit fails verification |
| Server policy | `ContactServerPolicy` | optional | validator limits apply (4 000 chars, subject optional) |
| Success redirect | `ContactSuccessRedirect` | required for the redirect | inline success with JS; page reload without it |

`leptos_routes_with_context` registers each server function at its own path
using this closure, so page renders and `submit_contact` share it.  A
hand-written `/api/{*fn_name}` route is unnecessary, and a value provided
only there does not reach the server function: Axum prefers the literal
path the framework registered.  Earlier revisions of this document described
two context sites; that was incorrect (RFC 007).

Where a value must differ by request kind, read `Parts` from context and
branch on the method or path rather than splitting the closure.

### 2.3 Rendering modes

| Mode | Component runs | Server function | Notes |
|------|----------------|-----------------|-------|
| SSR + hydrate | server, then browser | server | Full interactivity; the hidden token is rendered by SSR and preserved by hydration (tachys does not rewrite a reactive attribute on first hydration from server HTML).  A form reached by client-side navigation arrives with an empty token field and fetches one from `issue_form_token_fn` |
| SSR only, no WASM shipped | server | server | Plain POST; behaviour per §4.1.4 |
| Islands | server; island in browser | server | Same as SSR + hydrate for the island |

---

## 3. Data seen at the boundary

| Item | Direction | Contains PII | Where it lives |
|------|-----------|--------------|----------------|
| Form fields | browser → server | yes | request body only; never persisted [FR-OBS-04] |
| Field-error payload | server → browser | no (generic text) | server-fn error, or `__err` URL parameter in no-JS mode |
| Generic error text | server → browser | no | same |
| Email message | server → relay | yes | SMTP session |
| Log events | server → log sink | no [FR-OBS-02] | operator's logging |
| Token | server → browser → server | no | hidden field |

---

## 4. External interfaces

### 4.1 UI component `ContactForm`

#### 4.1.1 Props

| Prop | Type | Default | Purpose |
|------|------|---------|---------|
| `classes` | `ContactFormClasses` | all empty | CSS hook per structural element [FR-UI-03] |
| `labels` | `ContactFormLabels` | English | Every rendered string [FR-UI-02] |
| `options` | `ContactFormOptions` | subject shown, optional, 4 000 | UI behaviour [FR-UI-11] |

#### 4.1.2 DOM contract

The following identifiers and attributes are **public API** [NFR-COMPAT-05].

| Element | `id` | `name` | Fixed attributes | Class hook |
|---------|------|--------|------------------|------------|
| wrapper `<div>` | — | — | — | `root` |
| success `<div>` | — | — | `role="status" aria-live="polite"` | `success` |
| generic error `<div>` | — | — | `role="alert" aria-live="assertive"` | `error` |
| `<form>` (`ActionForm`) | — | — | `method="post" action="/api/submit_contact"` | — |
| field wrapper `<div>` ×4 | — | — | — | `field` |
| `<label>` ×4 | — | — | `for` = input id | `label` |
| name `<input>` | `contact-name` | `name` | `type=text required maxlength=80 autocomplete=name aria-required=true` | `input` |
| email `<input>` | `contact-email` | `email` | `type=email required maxlength=254 autocomplete=email aria-required=true` | `input` |
| subject `<input>` (if shown) | `contact-subject` | `subject` | `type=text maxlength=120`; `required`/`aria-required` follow `require_subject` | `input` |
| message `<textarea>` | `contact-message` | `message` | `required maxlength=<options> rows=6 aria-required=true` | `textarea` |
| field error `<p>` | `<input-id>-error` | — | `role="alert" aria-live="polite"` | `error` |
| token `<input>` | — | `form_token` | `type=hidden` | — |
| honeypot wrapper `<div>` | — | — | `aria-hidden=true`, off-screen inline style | — |
| honeypot `<input>` | `contact-website` | `website` | `type=text tabindex=-1 autocomplete=off` | — |
| submit `<button>` | — | — | `type=submit`; `disabled` and `aria-busy` while pending | `button` |

When a field has an error its input additionally carries
`aria-invalid="true"` and `aria-describedby="<input-id>-error"` [FR-A11Y-03].

Because ids are fixed, at most one `ContactForm` per page is supported.
Supporting several instances would require an id-prefix prop; not planned.

#### 4.1.3 State model

```text
                 submit
   ┌──────────┐ ───────▶ ┌──────────┐  Ok(())   ┌──────────────────────┐
   │   idle   │          │ pending  │ ────────▶ │ success              │
   │          │ ◀─────── │ button   │           │ form removed,        │
   └──────────┘  result  │ disabled │           │ status message shown │
     ▲   ▲   ▲           └──────────┘           └──────────────────────┘
     │   │   │                │
     │   │   └── Err(field payload) ──▶ idle + per-field errors
     │   │                                (inputs preserved: target, P-10)
     │   │                                (focus → first invalid: target, P-16)
     │   └────── Err(other) ──────────▶ idle + generic banner
     └────────── visitor edits (errors stay until next submit)
```

| State | Visible | Announced | Current status |
|-------|---------|-----------|----------------|
| idle | form | — | Met |
| pending | form, button disabled, text = `labels.sending` | `aria-busy` | Met |
| success | success message only | polite | Met with JS; no-JS Gap (P-13) |
| field-error | form, message under each failed field | polite alert per field | Met (JS); no-JS reload loses input by design |
| generic-error | form + banner with `labels.error` | assertive | Met |

Design rule: the form element and its inputs MUST be created once and kept
across result changes; only the error and success regions react.  Met since
RFC 002 handoff 02: the form closure depends on the success state alone, so
a validation error rebuilds nothing and the SSR-rendered hidden token
survives (P-11 closed).  Per-field errors and their ARIA attributes react
through individual attribute closures.

#### 4.1.4 Behaviour with and without JavaScript

| Event | With WASM | Without WASM (plain POST) |
|-------|-----------|---------------------------|
| Submit | `fetch` POST, form-encoded; page stays | Browser POST; server answers `302` to the configured success page, or to the Referer when none is configured |
| Success | success page when `ContactSuccessRedirect` is in context (client-side navigation), otherwise the inline success state | `302` to the success page when one is configured; without one the page reloads with no confirmation, which is why configuring one is recommended |
| Field errors | payload parsed by variant, shown per field | framework appends `__err=<encoded>` to the Referer; SSR renders the action value from it and shows field errors [FR-PE-02]; input is lost by the reload |
| Generic error | banner | same `__err` mechanism → banner |
| Token | hidden field from SSR; survives re-render since RFC 002 handoff 02 | fresh token on every render; always valid |

### 4.2 HTTP interface

| Property | Value |
|----------|-------|
| Path | `/api/submit_contact` (fixed by `endpoint = "submit_contact"`; the `/api` prefix is the Leptos default) |
| Method | `POST` |
| Request content type | `application/x-www-form-urlencoded` (Leptos default `PostUrl` encoding, identical for `<form>` and `fetch`) |
| Response, WASM client | server-function encoding of `Result<(), ServerFnError>` |
| Response, browser form | `302` redirect to the Referer; on error the framework appends `__err` |
| HTTP status on error | determined by the Leptos server-function runtime; clients MUST rely on the encoded error, not on the status code |

#### 4.2.1 Request fields

| Field | Required | Server treatment |
|-------|----------|------------------|
| `name` | yes | trim; 1–80 chars; no CR/LF |
| `email` | yes | trim; valid syntax; ≤ 254 chars; domain of at least two labels, none empty; no address literal (`[…]`) |
| `subject` | no | trim; blank → absent; ≤ 120 chars; no CR/LF; may be required by policy |
| `message` | yes | trim; 1–4 000 chars; policy may lower the ceiling |
| `website` | must be empty | non-empty → honeypot: success response, no delivery |
| `form_token` | when `form-token` enabled | verified before anything else; absent = invalid |

Unknown fields are ignored by the deserialiser.  Field order is irrelevant.

#### 4.2.2 Error classes

The server sends codes, never visitor-facing text.  Every message below is
rendered on the client from `ContactFormLabels::errors` [FR-I18N-02].

| Situation | Variant | String on the wire | Client rendering |
|-----------|---------|--------------------|------------------|
| Field validation or policy failure | `ServerFnError::Args` | `field_errors:{…}`, see §4.2.3 | per field, from `errors.required` / `length` / `format_email` / `format` / `line_breaks` |
| Token invalid, expired or missing | `ServerFnError::Args` | `contact_error:token_invalid` | banner, `errors.token_invalid` |
| Token config missing | `ServerFnError::ServerError` | `contact_error:not_configured` | banner, `errors.not_configured` |
| Delivery context missing | `ServerFnError::ServerError` | `contact_error:not_configured` | banner, `errors.not_configured` |
| Delivery failed | `ServerFnError::ServerError` | `contact_error:delivery_failed` | banner, `errors.delivery_failed` |
| Delivery timed out | `ServerFnError::ServerError` | `contact_error:delivery_timeout` | banner, `errors.delivery_timeout` |
| Unexpected | `ServerFnError::ServerError` | `contact_error:unexpected` | banner, `errors.delivery_failed` |

An unrecognised code — from a newer server — yields no match and the client
falls back to `labels.error`.

#### 4.2.3 Field-error payload protocol

The `Args` message is the sentinel `field_errors:` followed by compact JSON
with four optional members, each a `FieldError`:

```json
field_errors:{"name":{"kind":"length","min":1,"max":80},"email":{"kind":"format"}}
```

`FieldErrorCode` is internally tagged on `kind`, snake_case:
`required`, `length` (with `min` and `max`, in characters), `format`,
`line_breaks`.

`FieldError` is **untagged**, so a member may be either that object or a
plain string.  That is the compatibility hinge:

| Server | Client | Result |
|--------|--------|--------|
| current | current | code, rendered from labels |
| 0.3 (sentences) | current | parsed as `FieldError::Text`, shown unchanged |
| current | 0.3 | the object fails to parse as a string, so the whole payload is rejected and the 0.3 client shows its generic banner — which is what it did for every validation error anyway |

The client matches the `Args` variant
(`ContactFieldErrors::from_server_fn_error`) rather than the framework's
`Display` output, which prefixes the payload with
`error deserializing server function arguments: `; the string parser remains
as a tolerant fallback.  `ContactErrorCode::from_server_fn_error` reads the
`contact_error:` prefix from either variant the same way.

Lengths in `length` are characters, matching the validator, the policy and
the textarea's `maxlength`.

### 4.3 Server-side integration interface

Covered by §2.2.  Additional guarantees:

- `submit_contact` never panics on missing context; it logs and returns an
  error.
- Context values are read once per request; no global state.
- `ContactServerPolicy` can only tighten limits.  Values above the hard
  ceiling `MESSAGE_MAX_LEN` are clamped to it (M1).

### 4.4 Delivery interface

#### 4.4.1 Trait contract

`ContactDelivery::deliver(&self, input: ContactInput) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>>`

| Guarantee | Detail |
|-----------|--------|
| Input | already trimmed, validated, honeypot-checked, policy-checked [FR-DEL-02] |
| Concurrency | may be called concurrently; implementations hold no per-call mutable state |
| Errors | four categories: `Configuration`, `Transport`, `MessageBuild`, `Internal`; detail is logged by the crate, never shown to the visitor |
| Error text | logged verbatim for operators: the category and transport detail (status codes, the relay's reply), never the submission — no name, email address, subject, message, token or credential [FR-OBS-02, FR-OBS-03] |
| Time | bounded (0.6.0, RFC 009): the SMTP backend stops at `SmtpConfig::timeout` (default 30 s, connect through the final reply); any other backend is bounded by wrapping it in `DeliveryTimeout`.  On expiry the delivery is dropped and the visitor sees `delivery_timeout`.  A queue adapter remains a Future item [FR-DEL-08] |
| Cancellation | a delivery may be cancelled at any `.await` when wrapped in a timeout; implementations do not leave shared state half-updated across an `.await` |

#### 4.4.2 Email message specification (SMTP backend)

| Header / part | Value | Source |
|---------------|-------|--------|
| `From` | `from_address` | server config only |
| `To` | `to_address` | server config only |
| `Reply-To` | `"<name>" <email>` built with `Mailbox::new`, display name encoded per RFC 5322 / RFC 2047 | visitor (validated) |
| `Subject` | `<subject_prefix> <subject or "(no subject)">`, both CR/LF-sanitised | config + visitor |
| `Content-Type` | `text/plain; charset=utf-8` | fixed |
| Body | see template | visitor |

Body template:

```text
New contact form submission
===========================

Name:
<name>

Email:
<email>

Subject:
<subject or "(none)">

Message:
<message>
```

The visitor's address is never used in `From` (SPF/DKIM alignment and
anti-spoofing).  Message body text is not sanitised beyond the length limit
because it is body content, not a header.

#### 4.4.3 Backend behaviour matrix

| Backend | Feature | Transport | TLS modes | Logs |
|---------|---------|-----------|-----------|------|
| `NoopDelivery` | none | discards | — | `debug` "discarding contact form submission" |
| `LettreSmtpDelivery` | `smtp-lettre` | SMTP via `lettre`, tokio, native TLS | `StartTls` (587, default), `Tls` (465), `DangerousPlaintext` | `info` on success, `error` with transport detail on failure |
| custom | — | integrator-defined | — | integrator-defined; MUST follow [FR-OBS-02] |

### 4.5 Configuration interface

#### 4.5.1 Feature flags

| Flag | Implies | Adds |
|------|---------|------|
| `hydrate` | — | client hydration |
| `ssr` | — | server function body, SSR |
| `islands` | — | Islands mode |
| `smtp-lettre` | `ssr`, `delivery-timeout` | `delivery::smtp` |
| `delivery-timeout` | `ssr` | `delivery::timeout` (`DeliveryTimeout`) |
| `axum-helpers` | `ssr` | `axum_helpers` |
| `form-token` | `ssr` | `form_token` module, token field verification |
| `challenge-http` | `ssr` | `HttpChallengeVerifier` (vendor siteverify calls over HTTPS) |

`default = []`.  Features are additive; enabling one never removes an API.

#### 4.5.2 Typed configuration

| Type | Fields | Secret | Notes |
|------|--------|--------|-------|
| `SmtpConfig` | host, port, username, password, from_address, to_address, subject_prefix, tls_mode, timeout (`SmtpConfig::DEFAULT_TIMEOUT`, 30 s) | password (redacted in `Debug`) | never serialised; no `Default`, so every literal names `timeout` |
| `FormTokenConfig` | secret_key (≥ 32 random bytes recommended), ttl_secs (default 3 600), min_age_secs (default 2), binding | secret_key (redacted) | never serialised |
| `ContactServerPolicy` | require_subject, max_message_len | — | tighten-only |
| `ContactFormOptions` | show_subject, require_subject, max_message_len | — | UI only, not a security boundary |

#### 4.5.3 Environment variable conventions

The crate reads no environment variables [FR-SUB-10].  Examples and
documentation use these names consistently so integrators can copy them:

`SMTP_HOST`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`, `CONTACT_TO`,
`FORM_TOKEN_SECRET`, `ALLOWED_ORIGIN`, and optionally `CHALLENGE_PROVIDER`, `CHALLENGE_SITE_KEY`, `CHALLENGE_SECRET`.

### 4.6 Observability interface

| Level | Event | Structured fields | PII |
|-------|-------|-------------------|-----|
| `warn` | honeypot triggered | — | none |
| `debug` | validation failed | `name_err`, `email_err`, `subject_err`, `message_err` (booleans) | none |
| `warn` | form token rejected | `error` (the reason) | none |
| `debug` | token expired / future timestamp | `timestamp`, `now` | none |
| `error` | `FormTokenContext` not provided | — | none |
| `error` | `ContactDeliveryContext` not provided | — | none |
| `error` | delivery timed out | `limit_secs` | none |
| `error` | delivery failed | `error` (category + transport text) | none by contract; relays may echo addresses in SMTP replies, integrators SHOULD review log retention |
| `info` | delivered via SMTP | — | none |
| `debug` | no-op backend discarded submission | — | none |

A delivery error's `Display` text is logged verbatim in the `delivery failed`
event, so an implementation keeps it free of submission data (§4.4.1).

---

## 5. Security external design

### 5.1 Assets

SMTP credentials; token secret; operator inbox (its reachability and
reputation); visitor PII in transit; the application's availability.

### 5.2 Threat model

| # | Threat | Vector | Control | Owner | Status |
|---|--------|--------|---------|-------|--------|
| T1 | Email header injection | CR/LF in name or subject | validator rejects; `sanitize_header_value` at build; `Mailbox::new` encoding | crate | Met |
| T2 | Sender spoofing / SPF failure | visitor address as `From` | `From` always server-configured | crate | Met |
| T3 | Credential or recipient leak to client | serialising config into WASM or responses | config types exist only server-side; redacted `Debug` | crate | Met |
| T4 | Automated spam | bots posting the form | honeypot; token (proof of prior page fetch); rate limit; Turnstile | crate + app | Met (layers documented) |
| T5 | Cross-site request forgery | victim's browser posts from another origin | **Origin / Referer validation** (app) is the control; with `Binding::Cookie` the form token adds an independent double-submit check (0.5.0) | app + crate | Met; binding is defence in depth |
| T6 | Token replay | reuse of one token within TTL | not prevented by design (stateless); TTL bounds the window; rate limit bounds volume | — | Accepted risk pending P-12 |
| T7 | Flooding / resource exhaustion | many POSTs, large bodies | rate limit; 32 KiB body limit; 4 000-char message ceiling | app + crate | Met |
| T8 | Information disclosure through errors | stack traces or relay errors reaching the visitor | generic messages; details only in logs | crate | Met |
| T9 | PII leakage through logs | logging fields | no PII in events | crate | Met |
| T10 | Timing attack on token | byte-by-byte comparison | constant-time comparison | crate | Met |
| T11 | Clock skew abuse | far-future timestamps | 60 s future tolerance, TTL | crate | Met |
| T12 | Silent insecure misconfiguration | missing context, default secret | fail-closed; examples require env vars | crate + examples | Met |
| T13 | Stored XSS through the form | echoing input in HTML | input never echoed; Leptos escapes | crate | Met |
| T14 | Relay abuse as open relay | attacker-controlled `To` | `To` fixed by config | crate | Met |
| T15 | Slow relay holding connections | delivery without timeout | the SMTP backend's deadline (`SmtpConfig::timeout`, 30 s by default, covering the whole exchange); `DeliveryTimeout` for any other backend; the visitor sees `delivery_timeout` | crate | Met (0.6.0); residual: a custom backend not wrapped in `DeliveryTimeout` |
| T16 | Open redirect or header injection through the success page | a configured redirect path that leaves the site, or carries CR/LF into the `Location` header | `ContactSuccessRedirect::new` accepts only site-relative paths: it requires a leading `/`, rejects `//`, `\`, `://`, and every control or whitespace character, so neither an off-site target nor a header break survives construction.  The path is fixed at startup and never read from form input or a query parameter | crate | Met (0.4.0) |
| T17 | Cookie tossing defeats binding | a sibling subdomain sets the binding cookie with `Domain` and `SameSite=None`, paired with a token the attacker fetched for that nonce | `__Host-` cookie prefix, applied when the cookie is `Secure` on `/`; a second `__Host-` cookie from our own origin fails closed (`BindingMismatch`); Origin validation still rejects the POST | crate + app | Met at defaults (0.5.0); not with `secure: false` or a non-root path, documented |
| T18 | Detection oracle on silent outcomes | a bot compares the response to a honeypot hit or `SilentDrop` with a genuine submission's: with a success page configured, only the genuine one carried the redirect | every successful outcome applies the success redirect through one helper | crate | Met (0.5.0, `67c1ac1`); regressed in 0.4.0 |
| T19 | Vendor verify call leaks or stalls | a redirect from the verify endpoint (or a proxy set by `with_verify_url`) receives the request body, which carries the vendor secret; a slow or failing vendor holds the request | redirects disabled on the verify client, so a 3xx is `Unavailable`; 5-second timeout; every error fails closed (`challenge_unavailable`); secret redacted in `Debug`, absent from logs; `with_verify_url` is integrator configuration, never request input | crate | Met (0.5.0) |
| T20 | Abuse of the public token endpoint | scripted `POST /api/form_token` to mint tokens or flood the server | a minted token grants nothing a page render does not; the endpoint sits behind the router-wide Origin check (`403` cross-origin) and rate limit (`429` after the burst, verified 2026-09-13); the browser calls it only when `token_refresh_secs` is set, and never in a loop | app + crate | Met (0.5.0) |

New in 0.4.0: the success redirect (T16) is the crate's first outward
response header, so it is the first place a configuration value reaches a
protocol boundary.  Validation happens once, at construction, rather than
at each use.

### 5.3 Anti-abuse layering

Ordered from the edge inward; each layer removes cheap attacks before more
expensive checks run.

1. TLS termination and WAF (proxy).
2. Request body limit (app layer).
3. Rate limit keyed by client IP (app layer).
4. Origin / Referer strict match on POST (app middleware). **This is the
   CSRF control.**
5. Form token (crate, `form-token` feature): proves the sender fetched a page
   from this server within the TTL; **target** also enforces a minimum age
   since render (RFC 004).
6. Honeypot (crate).
7. Field validation and server policy (crate).
8. Optional challenge verification (crate, `challenge` feature, RFC 005):
   the only layer that needs JavaScript; no-JS submissions rejected by
   default when enabled.
9. Optional pre-delivery filter hook (crate, RFC 006).

### 5.4 Anti-forgery token: current design and decision

**Current.** `{unix_seconds}|{16-byte nonce hex}|{HMAC-SHA256 hex}`, signed
with `secret_key`, valid for `token_ttl_secs`, constant-time verified.  It
is issued to any SSR request and is not tied to a cookie, session, or IP.

**What it proves.** The bearer obtained a token from this server within the
TTL.  **What it does not prove.** That the bearer is the visitor whose
browser is now submitting.  An attacker can fetch a token and embed it in a
cross-site form; the victim's browser will submit it and verification
succeeds.  For an unauthenticated contact form the harm of such forgery is
low (the attacker could post directly), which is why the honest description
is "anti-automation", not "CSRF protection".

**Options for the RFC (P-12).**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A. Cookie binding (double submit) | Also set a `SameSite=Lax`, `HttpOnly` cookie containing the nonce (or a signed value); verify hidden field and cookie agree | Real CSRF property; stateless | Needs response-header access: feasible in `axum-helpers`, not in the framework-neutral core; cookie consent considerations |
| B. Session binding | Bind token to an application session id supplied via context | Strongest | Requires the app to have sessions; most static sites do not |
| C. Reposition and rename | Keep the token as an anti-automation "form token"; document Origin validation as the CSRF control; rename feature/API in a minor release with deprecation aliases | Honest, cheap, keeps the useful bot friction | Rename churn |
| D. Remove | Drop the feature | Simplest | Loses useful bot friction |

**Decision (owner, 2026-09-12, RFC 004).** C and A together: the feature is
renamed *form token* with deprecated aliases for one minor; a minimum age of
two seconds is enforced; cookie binding is available as an opt-in in
`axum-helpers`; a form created in the browser acquires a token from a server
function.

### 5.5 Data classification and retention

| Data | Class | Retention by crate |
|------|-------|--------------------|
| name, email, message | PII | none; passes through to delivery |
| client IP | PII | never seen by the crate; used by the app's rate limiter |
| token | non-secret, short-lived | none |
| secrets | secret | in-memory config only |

---

## 6. Internationalisation design

| Aspect | Current | Target |
|--------|---------|--------|
| Component strings | `ContactFormLabels`, all overridable | unchanged; add presets (P-20), for example `ContactFormLabels::ja()` |
| Server-originated messages | codes on the wire (§4.2.3); the component renders them from `labels.errors` (RFC 003) | unchanged |
| Text direction | not set by the component | unchanged; integrator sets `dir` on the host page or wrapper class |
| Length limits | characters everywhere (M1) | unchanged |
| Email header encoding | RFC 2047 via lettre | unchanged |
| Language attribute | not set | unchanged; belongs to the page |

Design rule: the crate never chooses a language.  It renders whatever
strings it is given, and never composes visitor-facing text on the server.

---

## 7. Accessibility contract

The full behaviour is in [Accessibility](../guides/accessibility.md).  The
contract, in one table:

| Aspect | Guarantee |
|--------|-----------|
| Labels | explicit `<label for>` for every control |
| Required | `required` + `aria-required` |
| Errors | `aria-invalid`, `aria-describedby`, polite `role="alert"` per field |
| Status | polite live region on success, assertive on generic error |
| Busy | `aria-busy` and text change on the button |
| Honeypot | `aria-hidden`, out of tab order, off-screen |
| Focus | native outlines untouched; **target**: focus to first invalid field after failure (P-16) |
| Colour | none shipped; state always in text |

---

## 8. UX design principles applied

The project rule is "less is more".  Applied here:

- **Three required fields and one optional.**  Nothing else is shown by
  default.  Phone, company, consent checkboxes are deliberately absent; an
  integrator who needs them wraps the component or waits for the slot API
  (Future).
- **One primary action.**  The button is the only control besides inputs.
- **Feedback where the eye is.**  Field errors sit under their field; the
  success message replaces the form so the visitor cannot double-submit.
- **Never punish the visitor for a server-side decision.**  Typed input is
  preserved on error (target); the token never expires under a visitor who
  took a while to write (one-hour TTL by default).
- **No CAPTCHA by default.**  Friction is added only when the integrator
  decides the threat justifies it; layers are opt-in in documented order.
- **Advanced controls exist for mature integrators** (policy, token, custom
  delivery) but are invisible to a first-time integrator following the
  Quick Start.

---

## 9. Compatibility and versioning

| Change | Classified as |
|--------|---------------|
| New optional prop, new feature flag, new label field with a default | non-breaking |
| Change to element ids, field names, class hook names, ARIA attributes | breaking (minor in 0.x, with migration note) |
| Change to the error payload format | breaking unless the client accepts both forms for one minor release (§4.2.3) |
| Rename of the `form-token` feature or API | breaking; ship deprecation aliases for one minor (done once: renamed in 0.5.0, aliases removed in 0.6.0) |
| MSRV bump | minor |
| Defect fix that changes observable behaviour to match this document | patch |

---

## 10. Pending design decisions

| ID | Topic | Options | Architect recommendation | Decides |
|----|-------|---------|--------------------------|---------|
| P-12 | Anti-forgery token | — | **Decided 2026-09-12:** C + A, see §5.4 and RFC 004 | — |
| P-10/11/13/16 | Form state model | (i) keep single closure, stash inputs in signals; (ii) build form once, react only in error/success regions; (iii) controlled inputs | (ii): smallest change that satisfies FR-UI-07, FR-UI-12 and keeps no-JS identical | architect via RFC |
| P-13 | No-JS success signal | (i) redirect to an integrator-configured success page; (ii) marker query read at SSR; (iii) cookie; (iv) document limitation | (i): framework-neutral contract, no URL parsing in the component, integrator owns the page | architect via RFC 002 |
| P-14 | Error codes | (i) codes in existing JSON members; (ii) new JSON shape with version key | (i) with dual-accept period | architect via RFC |
| P-23 | Cloudflare Workers | in scope / out of scope | out of scope for 0.4; revisit with P-22 adapters | owner |
| P-21 | Challenge providers | — | **Decided 2026-09-12:** in scope; Turnstile, hCaptcha, reCAPTCHA v2/v3; no-JS rejected by default with opt-in; `challenge-http` feature.  Remaining design in RFC 005: verify timeout and outage behaviour, widget theming and localisation, score threshold for v3 | architect via RFC 005 |
| FR-VAL-08 | Message ceiling | constant 4 000 / configurable with documented max | keep constant; raise only on evidence | owner |

---

## 11. Traceability

| Requirement group | Design sections |
|-------------------|-----------------|
| FR-UI | 4.1 |
| FR-SUB, FR-VAL | 4.2, 4.3 |
| FR-ABUSE | 1.2, 5.2, 5.3, 5.4 |
| FR-DEL | 4.4 |
| FR-CFG | 2.2, 4.5 |
| FR-I18N | 6 |
| FR-A11Y | 4.1.2, 7 |
| FR-PE | 2.3, 4.1.4 |
| FR-OBS, NFR-PRIV | 3, 4.6, 5.5 |
| NFR-SEC | 5 |
| NFR-COMPAT | 4.1.2, 9 |
| NFR-PORT | 1, 10 |

---

## 12. Change history

| Date | Version | Change |
|------|---------|--------|
| 2026-09-12 | Draft 1 | Initial external design from architect baseline review of `0.3.3` |
| 2026-09-13 | Draft 8 | 0.5.0 security audit: T19 vendor verify call, T20 public token endpoint |
| 2026-09-13 | Draft 7 | T18: silent outcomes must end like success (regression from 0.4.0) |
| 2026-09-13 | Draft 6 | RFC 004 handoff 02: T5 updated for binding; T17 cookie tossing and the `__Host-` prefix |
| 2026-09-13 | Draft 5 | RFC 004 handoff 01: the token is named *form token* throughout; the hidden field is `form_token`; minimum age recorded |
| 2026-09-13 | Draft 4 | 0.4.0 security audit: T16 added for the success redirect, the release's only new outward data flow |
| 2026-09-12 | Draft 3 | M1 outcomes marked current (§4.1.3, §4.1.4, §4.2.3, §4.3, §6) |
| 2026-09-12 | Draft 2 | Anti-abuse theme decisions folded in (§1.2, §5.3, §10); no-JS success target revised to a configured success page |
