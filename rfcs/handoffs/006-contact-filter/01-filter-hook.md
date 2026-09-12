# Handoff 01 — Filter hook and the layers table

**RFC.** [006](../../accepted/006-contact-filter.md), D1–D4.
**Requirements.** FR-ABUSE-14, FR-OBS-02.

## Purpose

Add the last anti-abuse layer and, at the same time, make the five layers
read as one design for developers.

## Change scope

New `src/filter.rs` + `src/filter/tests.rs`; `src/server.rs`;
`src/error.rs` (`Rejected`); `src/config.rs` (`rejected` label);
`src/lib.rs`; docs: new `security/filter.md`, `security/README.md`,
`reference/api.md`, `guides/localization.md`, `SUMMARY.md`, `CHANGELOG.md`.

## Explicit non-change scope

No built-in filters; no request metadata; no change to earlier steps.

## Required implementation

1. **`filter.rs`** (under `ssr`, no feature flag): exactly the API in RFC
   D1.  `FilterChain::new(Vec<Arc<dyn ContactFilter>>)`; its `filter`
   awaits each in order and returns the first non-`Accept`; its `name()`
   returns the name of the filter that decided (store it in a
   `tracing::Span` field or return it through an internal method used
   for logging; keep the public trait as specified).
2. **Codes and labels.**  `ContactErrorCode::Rejected` (`rejected`);
   `ContactErrorLabels.rejected = "Your message could not be accepted."`.
3. **`server.rs`.**  After the challenge step, before delivery:

   ```rust
   if let Some(filter) = use_context::<ContactFilterContext>() {
       match filter.filter(&input).await {
           FilterDecision::Accept => {}
           FilterDecision::Reject => { tracing::warn!(filter = filter.name(), "submission rejected by filter"); return Err(ServerFnError::Args(ContactErrorCode::Rejected.into_server_fn_message())); }
           FilterDecision::SilentDrop => { tracing::warn!(filter = filter.name(), "submission silently dropped by filter"); return Ok(()); }
       }
   }
   ```

4. **Docs — `security/filter.md`.**  Sections: what a filter is and is
   not (one paragraph contrasting with server policy and challenge);
   the API; `MaxLinks` example as a compiling doctest; `BlockedDomains`
   example; external classifier outline with timeout and the fail-open /
   fail-closed choice written out; chaining; a privacy note that filters
   see PII and must not log it.
5. **Docs — `security/README.md`.**  Insert the D4 table under a heading
   "Which layer decides what" directly after "Who does what"; every row's
   mechanism links to its page.  Remove any duplicated explanation of a
   mechanism from this page so each has one home.
6. **API, localization, SUMMARY, CHANGELOG.**  As usual.

## Required tests

`filter/tests.rs`: a filter returning each decision; chain returns the
first non-`Accept` and does not call later filters (use counters); empty
chain accepts; default `name()` is the type name.
Doctest: the `MaxLinks` example compiles and, called on a three-link
message with `max: 2`, returns `Reject`.

## Acceptance criteria

- Tests and doctests pass; gates green.
- On the example, a filter rejecting messages containing `viagra` (test
  only, not committed to the example) returns the generic banner; a
  silent-drop filter returns success with no delivery (log line shows the
  filter name).  Transcripts pasted.
- Reading `security/README.md`, a developer can tell in one table which
  mechanism to configure for a given problem; the reviewer will read it
  with that question in mind.

## Prohibited shortcuts

Synonyms for the three decisions; a built-in filter in the crate; passing
raw arguments to filters; logging message content.

## Compatibility and security constraints

Additive.  Filters run only on validated input.

## Known risks

None significant.

## Required evidence

Gate outputs; tests; transcripts; the rendered overview table.
