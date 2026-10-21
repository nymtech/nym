# Tasks: Add Semver Release Gate

## 1. Semver gate in the dry-run workflow

- [x] 1.1 Add an "Install cargo-semver-checks" step (`cargo install cargo-semver-checks`, matching the existing cargo-workspaces install) after the "Bump versions (local only)" step
- [x] 1.2 Add a "Semver check (release gate)" step that computes the `nym-sdk` dependency closure from the lockfile (`cargo tree`) and runs `cargo semver-checks` over it, excluding closure crates with no crates.io baseline (currently `nym-config-score`)
- [x] 1.3 Keep the cheap preflight before the gate, so a broken manifest fails in seconds; the gate still runs after the version bump
- [x] 1.4 Raise the job timeout from 35 to 90 minutes
- [x] 1.5 Add a `Swatinem/rust-cache` step so reruns are warm

## 2. Calibration run (user-run, needs network and a runner, blocks relying on the gate)

- [x] 2.1 Verify detection locally: `cargo semver-checks -p nym-crypto --baseline-rev fb56bcf720` goes red on the real break (feature removal). Done 2026-10-21.
- [ ] 2.2 Run the scoped check locally and collect any closure crate with no crates.io baseline (reported as "not found in registry"); confirm the exclude list beyond `nym-config-score`
- [ ] 2.3 Dispatch the dry-run with a known-breaking change under a minor or patch number and confirm the job goes red
- [ ] 2.4 Dispatch with the matching major number and confirm the job goes green
- [ ] 2.5 Measure the real cold job duration and set `timeout-minutes` from it

## 3. Maintenance and tuning

- [ ] 3.1 Remove a crate from the `exclude` list once it has had its first crates.io release (otherwise it is silently unchecked)
- [ ] 3.2 Add targeted `--features` for public API behind non-default features, if the default feature set misses a surface that matters
- [ ] 3.3 If the closure check is still too slow, scope further to crates changed since the last release

## 4. Documentation

- [ ] 4.1 Note in the crate-release runbook that a breaking change requires a major version bump, and that the dry-run gate enforces it
- [ ] 4.2 Note the gate's blind spot: behavioural breaks behind an unchanged signature are not caught

## 5. Out of scope

- Remediation of the already-published `1.22.x` crates. Tracked separately as a one-off release decision.
- A per-pull-request semver canary. Rejected as too noisy, because mid-cycle the manifest version equals the last release, so every breaking pull request would read as an insufficient bump.
- Pinning intra-workspace dependencies with `=`. Not needed once version numbers are honest, since caret is the correct range for honest-semver crates.
- Making the gate unskippable by moving it into `ci-crates-version-bump.yml`. Deferred.
