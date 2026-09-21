# Handoff 016-02 — `--locked` examples in CI, and pins Dependabot cannot move

**RFC.** [RFC 016](../../done/016-msrv-and-ci-hygiene.md) D2, and the review of handoff 01 r2 (notes 1 and 2)
**Roadmap.** P-24
**Depends on.** 01 r2 (approved)

## Goal

- **A stale example lock fails at push time,** not at the release.
- **Every pin Dependabot can move, it moves;** the one it cannot is on a
  checklist.

## Change scope

### 1. `--locked` in the CI `examples` job

- **Both steps** of the job (`check` and `check (wasm client)`) get
  `--locked`.
- **Why, in a comment:** a change to the crate's dependencies changes the
  examples' locks, and only `--locked` notices.  Handoff 01 made `subtle` a
  direct dependency and the miss survived CI (`cff320e`).
- **Check it locally first,** with the current locks: both steps pass.  If
  either needs a refresh, refresh the lock in the same commit and report the
  diff.

### 2. An exact version comment on the `install-action` pin

- **Today:** `taiki-e/install-action@26e9283… # v2`.
- **The problem:** Dependabot's first pull request moved `actions/checkout`,
  whose comment was `# v6`, but left this one.  A comment that names a
  moving major tag does not tell it what the pin's version is.
- **The change:** the comment becomes the exact release the SHA belongs to.
  - **Find it:** `gh api repos/taiki-e/install-action/tags` or the commit's
    own release; the pinned commit is "Release 2.87.13" (2026-09-15).
  - **Write:** `# v2.87.13`.
- **Do not change the SHA** in this handoff.
- **Report** whether Dependabot then proposes an update; if its next pull
  request moves this pin, say so in the request or in a follow-up note.

### 3. `dtolnay/rust-toolchain` — a release-process step

- **Why it cannot be automated:** that repository has no version tags for
  the action itself; the pin is a `master` commit, and Dependabot leaves it
  alone.
- **The change:** in `docs/src/development/release-process.md`, in the
  pre-release checks, add one step:
  - **Check the pin:** compare the pinned SHA with
    `gh api repos/dtolnay/rust-toolchain/git/refs/heads/master`.
  - **If it moved:** read the diff, and update the pin in a commit of its
    own, with the date in the comment
    (`# master, checked YYYY-MM-DD`).
  - **If it did not:** say so in the release evidence.
- **Also update the comment now** to `# master, checked 2026-09-22`, so the
  first check has a date to compare against.

### 4. `CHANGELOG.md` `[Unreleased]`

Nothing.  This is CI and process only, with no effect on the published
crate.  Say so in the request.

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The examples with `--locked`,** both steps, locally.
- **CI** on the pushed commit: all six jobs, and the `examples` job's
  command lines showing `--locked`.

## Review request

`.git-exclude/review-request/016-msrv-and-ci-hygiene/02-locked-examples-and-pin-upkeep.md`:
- the commit;
- the CI diff;
- the `install-action` version and how you found it;
- the release-process step;
- the gates and CI.
