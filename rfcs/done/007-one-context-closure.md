# RFC 007 — One context closure for Axum

**Status.** Implemented (0.4.0) — released 2026-09-13, tag `0.4.0` on commit `c9a3f53`.  Written and accepted by the architect on 2026-09-12
as a correction of integration guidance (no security boundary or public
API change); owner informed in the RFC 002 handoff 03 review.
**Tracks.** Roadmap P-28.  Requirements FR-CFG-02, NFR-DOC-01.  External
Design §2.2.
**Touches.** Documentation (Quick Start, Axum Integration, Security pages,
Production Checklist, External Design §2.2), both examples, rustdoc of
`axum_helpers`, and the wording of RFC 004/005/006 handoffs.  No crate
logic.
**Handoffs.** [`../handoffs/007-one-context-closure/README.md`](../handoffs/007-one-context-closure/README.md)

## Summary

The crate has told integrators since 0.1.0 to register a manual
`/api/{*fn_name}` route with one context closure and to pass a second
closure to `leptos_routes_with_context`, and to provide every context value
in both ("the two context sites").  `leptos_axum` 0.8 registers every
server function at its literal path itself, using the closure given to
`leptos_routes_with_context`, and Axum prefers a literal path over a
wildcard.  The manual route is therefore never reached for registered
server functions.  The rule was wrong, and it cost a handoff a day of
confusion (RFC 002 handoff 03, §6).

This RFC replaces the rule with one: **provide everything in the single
closure passed to `leptos_routes_with_context`; do not register a manual
server-function route.**  Values that must differ between a page render
and a server-function call read `http::request::Parts`, which
`leptos_axum` provides before it calls the closure.

## Motivation

- Truthfulness: the documented pattern does not do what it says.
- Simplicity: one closure, one place to look, no duplicated `provide_context`
  lines.
- RFC 004 depends on knowing which closure serves which request; the
  honest answer is "the same one", so its design is amended here.

## Design

### D1 — The rule

```rust
let app = Router::new()
    .leptos_routes_with_context(&leptos_options, routes, move || {
        provide_context::<ContactDeliveryContext>(Arc::clone(&delivery));
        provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
        provide_context(generate_csrf_token(&csrf));      // unused on POSTs; harmless
        provide_context(success_redirect.clone());
    }, { let o = leptos_options.clone(); move || shell(o.clone()) })
    .fallback(leptos_axum::file_and_error_handler(shell))
    .with_state(leptos_options)
    .layer(/* security layers as before */);
```

No `.route("/api/{*fn_name}", …)`.  `delivery_context_fn` remains useful
as the delivery part of that closure; its rustdoc stops mentioning two
sites.

### D2 — Request-kind gating

Where a value must be produced only for page renders (RFC 004: issuing a
token cookie), the closure reads
`use_context::<http::request::Parts>()` and checks `parts.method == GET`.
`leptos_axum` provides `Parts` and `ResponseOptions` before calling the
closure in both the render handler and the server-function handler
(verified in 0.8.10, `lib.rs` lines 393–396 and 955–963).  This gating
lives in the `axum_helpers` functions that need it, never in integrator
code.

### D3 — The advanced case

An integrator who genuinely needs a different context for server
functions can exclude the automatic registration
(`leptos_axum` offers a `leptos_routes_with_exclusions…` family) and
register the route by hand.  Documented in one paragraph under "Advanced"
in Axum Integration; not the recommended path.

## Compatibility

Guidance only.  Integrators following the old pattern keep working (their
manual route is simply unused).  No API change.

## Security considerations

None new.  The layers (body limit, rate limit, origin check) are
router-wide `.layer()`s and are unaffected by which handler serves a
request.  Removing the dead route removes a false belief, which is a
security improvement in itself.

## Testing

The example is the test: after the sweep, `axum-with-security` has no
manual route and all evidence from RFC 002 handoffs 02 and 03 still
reproduces (token survives failures, `/thanks` on success, `__err` on
error).

## Acceptance criteria

No page, example, or handoff still instructs a manual server-function
route or "both context sites"; RFC 004 handoffs describe one closure with
`GET` gating; the example evidence above reproduces.

## Implementation boundaries

One handoff.  Precedes RFC 003's handoff (which touches the same docs) and
RFC 004.

## Release implications

Ships with 0.4.0 documentation; no code.
