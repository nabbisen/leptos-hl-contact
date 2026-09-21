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
   implemented and reviewed.  RFCs stay in `rfcs/accepted/` until the
   release is published; they move in step 12.
2. **Version.**  Bump `version` in the workspace `Cargo.toml` and in both
   example manifests, then run `cargo check` in each example directory so
   their lock files pick up the new path-crate version.
3. **Changelog.**  Move entries from *Unreleased* to the new version with
   the release date.  Add migration notes for any breaking change.
4. **Gates.**  All [gates](./testing.md#the-gates) green on `main`,
   including the feature-combination clippy step and the examples job.
   `cargo package` must not report a yanked crate in the lock file; if it
   does, `cargo update -p <crate>` and re-run the gates.
   **Check the toolchain action's pin.**  `dtolnay/rust-toolchain` has no
   version tags for the action itself, so the pin in `ci.yml` is a `master`
   commit that Dependabot never moves.  Compare it with
   `gh api repos/dtolnay/rust-toolchain/git/refs/heads/master`.
   - **If `master` moved:** read the diff, and update the pin in a commit of
     its own, with the date in the comment (`# master, checked YYYY-MM-DD`).
   - **If it did not:** say so in the release evidence, and update the date in
     the comment.
5. **Package.**  `cargo package -p leptos-hl-contact` succeeds without
   warnings.
6. **Security audit.**  If the release adds a data flow, an external
   integration, or authentication logic, update the threat model in
   [External Design](./external-design.md#5-security-external-design);
   otherwise confirm the existing controls still hold.
7. **Documentation check.**  Every page in this book describes the
   behaviour being released; `mdbook build` succeeds (its output
   `docs/book/` is ignored by git); no page outside `development/` names
   a past version, except a note that explains an upgrade or why older
   instructions differ (for example, the release that removed something); known issues are stated in the release-candidate request and,
   when they affect integrators, in the CHANGELOG; the README matches.
8. **Mutation run.**  Once per milestone, before the readiness report:
   `cargo mutants -p leptos-hl-contact --all-features` (output under
   `mutants.out/`, git-ignored).  The architect reads the surviving mutants
   and records in the report which matter and which are accepted.
   Informational, never a gate.
9. **Readiness report.**  Version, commit, included and excluded changes,
   test and build evidence, known issues, rollback note, mutation-run
   summary, recommendation.
10. **Owner approval.**
11. **Tag and publish.**  Tags are annotated (the repository forces
    signed-annotated tags): `git tag -a X.Y.Z -m "Release X.Y.Z"`, then
    `git push origin X.Y.Z` and `cargo publish -p leptos-hl-contact`.
    Publishing is irreversible; a version can only be yanked.
12. **Records.**  Move the milestone's RFCs to `rfcs/done/` with
    "Implemented (X.Y.Z)", fix inbound links, update `rfcs/README.md` and
    the roadmap, close the review folder for the milestone.
