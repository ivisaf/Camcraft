+++
title = "Releases"
description = "Versions, issues, pull requests and the release cut."
weight = 4
+++

## Versions

Versions follow semantic versioning, derived from the commits since the
last tag (Conventional Commits: `feat:`, `fix:`, `docs:` and so on):

| Commits since the last tag | Release |
|---|---|
| a breaking change (`feat!:`, `fix!:` or a `BREAKING CHANGE:` footer) | major (minor while the major is 0) |
| any `feat` | minor |
| only `fix`, `perf` or `refactor` | no release of its own; ships in the next minor |
| only `docs`, `test`, `ci`, `build` or `chore` | no release |

Only minor and major releases are cut, as `vX.Y.0` tags.

## From issue to main

Work is tracked as GitHub issues, picked by priority (`P0` wrong output,
gouge, crash or data loss; `P1` wrong behaviour a user hits; `P2` owed
polish) and weighted by effort (`effort:low` to `effort:max`).

1. **Claim** the issue with a comment before writing code.
2. **Branch** from the latest `main` (`issue/<N>-<short-name>`).
3. **Fix** it with a test that pins the change, and record any decision
   taken in `docs/decisions/<N>-<slug>.md`.
4. **Gate** it with `cargo xtask gate quick`.
5. **Open a draft pull request** that says what changed, how it was
   tested, what was not exercised, a `Changelog:` line, and ends
   `Fixes #N` (or `Refs #N` for partial work).
6. The PR is flagged ready once its gates hold, then **reviewed** by a
   reviewer that did not write it. `main` is protected: every change
   reaches it through a pull request, with merge commits only.

A `fix`, `perf` or `refactor` PR merges once its review is clean. A `feat`
or breaking PR merges at the next release cut.

## The release cut

1. **Review and merge** the waiting feature PRs into `main`.
2. **Release branch** `release/vX.Y.Z` from `main`:
   `cargo xtask decisions --version X.Y.Z` gathers the decision files into
   `RECOMMENDED.md`; the release notes and `CHANGELOG.md` rows are written;
   then `cargo xtask release --version X.Y.Z --no-tag` runs the release
   gate tier, bumps the version and builds the packages.
3. **Release PR** `chore(release): X.Y.Z`, reviewed and merged like any
   other.
4. **Tag** the release merge commit `vX.Y.Z` and push the tag. The tag
   starts `release.yml`, which builds every platform, publishes the GitHub
   release with its assets, and builds and publishes this site from the
   same tagged tree.

Each release overwrites the site, so it always documents the latest
release; older versions stay on the
[releases page](https://github.com/NeoTech/Camcraft/releases).
