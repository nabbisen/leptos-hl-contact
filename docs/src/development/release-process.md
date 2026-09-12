# Release Process

Releases are approved by the project owner.  The architect prepares a
release-readiness report; the dev team produces the candidate.

## Versioning

Semantic versioning in the `0.x` range:

| Release | May contain |
|---------|-------------|
| patch (`0.3.x`) | defect fixes, documentation; no public API or behaviour change |
| minor (`0.x.0`) | new features, breaking changes with a migration note, MSRV bumps |
| major | decided by the owner only |

The DOM contract (ids, field names, class hooks, ARIA) is public API.

Tags are `X.Y.Z` with **no** `v` prefix.

## Steps

1. **Scope.**  Confirm every roadmap item and RFC in the release is
   implemented and reviewed; move RFCs to `rfcs/done/` with the version in
   their Status field; update `rfcs/README.md`.
2. **Version.**  Bump `version` in the workspace `Cargo.toml` and in both
   example manifests.
3. **Changelog.**  Move entries from *Unreleased* to the new version with
   the release date.  Add migration notes for any breaking change.
4. **Gates.**  All four [gates](./testing.md#the-gates) green on `main`.
5. **Package.**  `cargo package -p leptos-hl-contact` succeeds without
   warnings.
6. **Security audit.**  If the release adds a data flow, an external
   integration, or authentication logic, update the threat model in
   [External Design](./external-design.md#5-security-external-design);
   otherwise confirm the existing controls still hold.
7. **Documentation check.**  Every page in this book describes the
   behaviour being released; `mdbook build` succeeds; the README matches.
8. **Readiness report.**  Version, commit, included and excluded changes,
   test and build evidence, known issues, rollback note, recommendation.
9. **Owner approval.**
10. **Tag and publish.**  `git tag X.Y.Z && git push --tags`, then
    `cargo publish -p leptos-hl-contact`.
11. **Roadmap.**  Mark released items, update milestone status.
