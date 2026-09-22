# Handoff 018-01 — Step 0 spike: DNS over HTTPS through the shared transport

**RFC.** [RFC 018](../../done/018-email-domain-check.md), Step 0
**Roadmap.** P-35
**Output.** A report.  **Nothing is committed to `main`.**  Spike code lives
on a local branch or in the scratchpad; paste the excerpts into the request.

## Goal

Find out whether **one** lookup path serves both targets, before any design
is fixed.

**Why this is a stop point.**  A Worker has no UDP, so ordinary DNS is not
available there.  If DNS over HTTPS cannot answer these questions through
`src/http/`, the feature either changes shape or goes back to the owner.

## What to try

- **The transport:** `src/http/`'s `HttpClient`, as RFC 017 left it.
- **The query:** DoH with the JSON media type, against **at least two**
  public resolvers, so the report is not a study of one vendor's quirks.
  Name which you used and when.
- **If a lookup needs a GET** and the shared client has only `post`, say so
  and describe the smallest change that would add one.  **Do not make that
  change in the spike.**

## The questions, each with evidence

| # | Case | What to report |
|---|------|----------------|
| a | a domain with an MX | the answer's shape, and the fields you would read |
| b | a domain with **no MX** but an A or AAAA record | whether the JSON lets you tell this apart from (c) |
| c | a domain with neither | as above |
| d | a **null MX** (`.`, RFC 7505) | whether it is distinguishable from (a) |
| e | NXDOMAIN versus SERVFAIL | the status codes, and that you can tell the visitor's mistake from ours |
| f | a resolver that does not answer | that the caller's time limit applies and the request is abandoned, on both targets |
| g | cost | bytes in and out, and the round trips per lookup |

**Both targets.**  Natively through the unit-test path; on wasm32 through
the worker suite's stubbed `fetch`, plus — if you can — one real lookup in
headless Chrome, and say which it was.

**Test domains:** use ones whose answers are stable and public.
`example.com` has no MX; pick a well-known domain for (a); RFC 7505 names
the null-MX shape.  **Do not use a real person's domain.**

## Stop conditions

Report and stop, without designing around them:
- the JSON shape cannot be parsed portably across the two resolvers;
- (d) or (e) cannot be distinguished;
- the shared transport cannot express the request without a change larger
  than adding a GET;
- a Worker build cannot reach the resolver at all.

## Review request

`.git-exclude/review-request/018-email-domain-check/01-step0-spike.md`:
- **Versions:** the crate's, and the resolvers with the date you queried;
- **Code:** the spike's diff excerpt;
- **The table above,** filled in, with the raw JSON for (a), (b), (d) and
  (e) — trimmed, with no address in it;
- **Cost** (g), and whether a GET is needed;
- **Your recommendation:** DoH as designed, or the alternative the results
  point to.

**Gates:** none, since nothing is committed.  Say that `main` is unchanged.
