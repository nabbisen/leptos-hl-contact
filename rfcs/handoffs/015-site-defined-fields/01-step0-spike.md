# Handoff 015-01 — Step 0 spike: a map argument through `server_fn`

**RFC.** [RFC 015](../../done/015-site-defined-fields.md), Step 0
**Roadmap.** P-43
**Output.** A report.  **Nothing is committed to `main`.**  Spike code lives
on a local branch or in the scratchpad; paste the relevant excerpts into
the review request.

## Goal

Before any design work, find out whether one map-shaped server-function
argument works in every path the form uses:

```rust,ignore
#[server(default)]
fields: Option<BTreeMap<String, String>>,
```

## What is known (architect, 2026-09-17)

- **The decoder.**  `server_fn` 0.8.13 decodes URL-encoded arguments with
  `serde_qs::Config::new(5, false)` (`src/codec/url.rs:61`, `:100`, `:139`).
  - **Depth:** 5.
  - **Strict mode off:** percent-encoded brackets (`fields%5Btopic%5D`),
    which browsers send, should parse.
  - **To prove:** both points, by the spike, not assumed.
- **The server suite harness** already posts both request forms:
  `Harness::submit_fetch` and `Harness::submit_nojs`
  (`tests/server/support/harness.rs`).

## Spike steps

On a local branch from current `main`:

1. **Add the argument** to `submit_contact`, last, as above.  Do nothing with
   it except count its entries into a test-visible place: for example, a
   `tracing` event with the **count only**, captured as the logging tests
   do.
2. **Server suite, both request forms** (`submit_fetch`, `submit_nojs`).  For
   each case below, report the HTTP status, what the visitor gets
   (delivered, the error code, or the `__err` redirect), and what the map
   contained: keys and count only, no values.

| # | Body addition | Question |
|---|---------------|----------|
| a | none | still delivered; map `None` |
| b | `fields[topic]=sales&fields[organisation]=Example` | two keys |
| c | `fields%5Btopic%5D=sales` (percent-encoded brackets, as browsers send) | one key `topic` |
| d | `fields[topic]=a&fields[topic]=b` (duplicate) | error, first, last, or list? |
| e | `fields[a][b]=x` (nested) | refused at deserialization?  What does the visitor see? |
| f | `fields[]=x` and `fields=x` | as (e) |
| g | `fields.a=x` | ignored, or a key? |
| h | `fields[ta%0Apic]=x` (control character in a key) | key as delivered, or refused? |
| i | 1,000 keys `fields[k0]…[k999]` | time taken, and whether depth or size limits apply |

3. **The browser: `ActionForm` with JavaScript.**
   - **The test:** in the browser suite, with the stubbed `fetch`, render
     inputs named `fields[topic]` (a text input) and `fields[timing]` (a
     `<select>`) inside the form.  Temporary markup in a test-only component
     is fine.
   - **Capture the request body** the client sends.
   - **Report:** whether it carries `fields[topic]=…`, with encoded or raw
     brackets, and whether the stubbed server-side decode of that exact body
     gives the map.
4. **The no-JavaScript error path.**
   - **The case:** force a validation error, for example a blank `name`,
     with (b) present.
   - **Report:** the `__err` redirect, and whether the page it lands on
     still renders.  No new error payload is needed yet: only whether an
     extra argument disturbs the redirect.
5. **Preservation.**
   - **Check:** how the built-in fields keep their input after a failed
     submission, with and without JavaScript.  Cite the code.
   - **Report:** whether the same mechanism would carry `fields[…]` values,
     or what would be needed.

## Stop conditions

Report and stop; do not design around them:
- (b) or (c) does not give the map in either request form;
- step 3's client body cannot be decoded into the map;
- (e) or (f) crashes, panics, or returns anything other than a refusal the
  form can show.

## Review request

`.git-exclude/review-request/015-site-defined-fields/01-step0-spike.md` must
include:
- **Versions:** `server_fn`, `serde_qs` and `leptos`, from `Cargo.lock`.
- **Code:** the spike's diff excerpt.
- **Results:** the table above, filled in for both request forms.
- **Steps 3–5:** the results, with citations.
- **Your recommendation:** keep the map shape, or the alternative the
  results point to (for example, one encoded string argument), with the
  reason.

**Gates:** none required, since nothing is committed.  Say that `main` is
unchanged.
