# RFC 006 — Pre-delivery filter hook

**Status.** Proposed — 2026-09-12.
**Tracks.** Roadmap M3 item P-25.  Requirement FR-ABUSE-14 (SHOULD).
External Design §5.3 layer 9.
**Touches.** New `filter.rs`, `server.rs`, `error.rs`, `config.rs`,
`lib.rs`, docs.

## Summary

A `ContactFilter` trait lets the integrator inspect a validated
submission just before delivery and accept it, reject it with a generic
message, or drop it silently.  The crate ships the hook and
documentation examples, not opinions about content.

## Motivation

Some spam passes every structural check: a valid address, a plausible
name, and a message that is three links.  Integrators want a place for
their own rules or a third-party classifier without forking the server
function.

## Goals

- One trait, one context, one decision enum.
- Ordered chains of filters.
- Silent drop indistinguishable from success (like the honeypot).
- Zero built-in rules in the crate.

## Non-goals

- Shipping heuristics (documented examples only).
- Giving filters the client IP or headers (no integration-neutral source
  yet; may follow the client-IP context if RFC 005's MAY is taken up).
- Async error policy configuration (see D2).

## Design

### D1 — API

```rust
pub enum FilterDecision { Accept, Reject, SilentDrop }

pub trait ContactFilter: Send + Sync + 'static {
    /// Called after validation, policy, and challenge; before delivery.
    fn filter(&self, input: &ContactInput) -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>>;
    /// Short name for logs; default = type name.
    fn name(&self) -> &'static str { std::any::type_name::<Self>() }
}
pub type ContactFilterContext = Arc<dyn ContactFilter>;

/// Runs filters in order; the first non-Accept decision wins.
pub struct FilterChain(Vec<Arc<dyn ContactFilter>>);
impl ContactFilter for FilterChain { … }
```

Provided in the server-function handler.  `submit_contact`: when the
context is present, `Reject` → `ServerFnError::Args(contact_error:rejected)`
with label `rejected` ("Your message could not be accepted."), `warn!`
with the filter name and no content; `SilentDrop` → `Ok(())`, `warn!`
likewise; `Accept` → deliver.

### D2 — Errors are the filter's business

The trait returns a decision, not a `Result`.  A filter that calls a
network service decides itself whether an outage means `Accept`
(fail-open, usual for spam classifiers: never lose a real enquiry) or
`Reject`.  The documentation states this rule and shows both patterns.
This keeps the crate free of an error-policy knob whose right default
differs per deployment.

### D3 — Documentation examples

`guides/filtering.md` (new): a `MaxLinks { max: 2 }` filter counting
`http://`/`https://` occurrences; a `BlockedDomains` filter on the email
domain; an outline for calling an external classifier with a timeout and
fail-open; chaining.  A note that filters see PII and must follow
FR-OBS-02 in their own logs.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Built-in heuristics behind options | Opinions age badly and differ per site; a hook is smaller and lasts |
| Filter before validation | Filters would see unvalidated input and duplicate checks |
| `Result<FilterDecision, E>` with an on-error policy | Adds configuration whose right default is deployment-specific; the implementor already knows their service |
| Access to request metadata | No neutral source; deferred |

## Compatibility

Minor, additive.

## Security considerations

Filters run with a validated input only.  Silent drop mirrors the
honeypot so bots learn nothing.  Rejection text is generic.  No new data
leaves the process unless a filter sends it; the documentation says so.

## Testing

Unit: mock filters returning each decision; chain ordering and
short-circuit; `name()` default.  `server` mapping of `Reject` and
`SilentDrop` covered by the integration tests that RFC 001 / P-15 add;
until then by the sentinel tests.

## Acceptance criteria

FR-ABUSE-14 Met; the documented `MaxLinks` example compiles as a doctest.

## Implementation boundaries

One handoff after RFC 005 handoff 1 (so the processing order is settled
once).

## Open questions

None.

## Release implications

`0.5.x`.
