# RFC 020 — Test follow-ups, and translations as contributed examples

**Status.** Accepted — 2026-09-22, as proposed: the tests, and translations
as contributed documentation rather than shipped presets.  Milestone M8
(P-38, and P-20 reshaped).
**Tracks.** Roadmap P-38, P-20.  Requirements NFR-TEST-*, FR-I18N-02.
**Handoffs.** [`../handoffs/020-test-and-docs-follow-ups/README.md`](../handoffs/020-test-and-docs-follow-ups/README.md)
**Touches.** tests only, plus `docs/src/guides/localization.md`,
`docs/src/guides/customization.md`, and the traceability table.

## Summary

Two small pieces that share a handoff shape and change no behaviour:
- **P-38:** the tests the mutation runs have been asking for since 0.6.0.
- **P-20, reshaped:** translations belong in the documentation as complete
  examples a reader can copy, not in the crate as shipped presets.

## D1 — The tests (P-38)

Each one exists because a mutation survived, and each pins behaviour a
reader of the requirements would expect to be pinned.

| Test | What it pins | First seen |
|------|--------------|------------|
| the form token at exactly its TTL, and one second past | the expiry boundary | 0.6.0 |
| the token at exactly the future-skew limit | the skew boundary | 0.6.0 |
| a `Debug` assertion for `FormTokenIssuer`, `ChallengeContext`, `FilterChain` and `ResendDelivery` | that each redacts or omits what it should | 0.6.0, 0.9.0 |
| `provide_contact_delivery` puts the delivery in context | the helper does what it says | 0.6.0 |
| the `challenge unavailable` and missing-context log events | the two logging-only arms | 0.6.0 |
| **a 64,000-byte response body is accepted** | the response cap's *value*, which the boundary tests cannot see because they build their bodies from the constant | 0.9.0 |

**Rules:** no production code changes; every test cites its requirement or
threat ID; each one is shown to fail against the mutation it kills.

## D2 — Translations as contributed examples (P-20)

**What P-20 asked for:** label presets shipped in the crate.

**Why not.**
- **We cannot verify the wording.**  The strings are a security-adjacent
  interface ("The security check did not pass"), and a plausible-but-wrong
  translation is worse than none.
- **The commitment compounds.**  Every new error code would need a
  translation in every shipped language, at the same moment, or the preset
  is silently half-English.
- **The saving is small:** about twenty short strings, once.

**What instead.**
- **The Localization guide keeps its complete, copyable example** and gains a
  short section explaining how to contribute another language: one file, all
  fields, the contributor's own language, and no partial sets.
- **A translated example is documentation,** so it carries no compatibility
  promise and no crate-version coupling.
- **The guide states plainly** that the crate ships English defaults and
  that every visible string is the site's to set (FR-UI-02, FR-I18N-02).

## D3 — Documentation lines from the reflerd.com letter of 2026-09-22

- **Customization, site fields:** keep a choice label short enough to read
  inside a phone's `<select>`; the label is what a visitor sees, the key is
  what delivery records.  (Their 320 px phone truncated a long label — a
  native select's behaviour, not ours.)

## Compatibility

None.  Tests and documentation only.

## Handoffs (planned)

| # | Scope |
|---|-------|
| 01 | D1, the tests |
| 02 | D2 and D3, the documentation |

## Owner decisions (2026-09-22)

Accepted as proposed; the questions as put, with the answers, were:

1. **P-20 as documentation, not presets** (D2).  The alternative is shipping
   translations we cannot check and must keep complete forever.
2. **No new language added by us** in this RFC: the guide gains the
   contribution path, not a second translation written by the architect.
