# Tasks: Add Semver Release Gate

## 1. Semver gate in the dry-run workflow

- [x] 1.1 Add an "Install cargo-semver-checks" step (`cargo install cargo-semver-checks`, matching the existing cargo-workspaces install) after the "Bump versions (local only)" step
- [x] 1.2 Add a "Semver check (release gate)" step running `cargo semver-checks --workspace`, so it inspects every published crate against its crates.io baseline with the target version already applied
- [x] 1.3 Keep the cheap preflight before the gate, so a broken manifest fails in seconds; the gate still runs after the version bump
- [x] 1.4 Raise the job timeout from 35 to 90 minutes

## 2. Calibration run (user-run, needs network and a runner, blocks relying on the gate)

- [ ] 2.1 Dispatch the dry-run once and collect the crates that error: wasm-only crates that will not build on the native runner, and any publishable crate with no prior crates.io release (both are hard errors, not skips)
- [ ] 2.2 Add those crates to `--exclude` on the semver step
- [ ] 2.3 Dispatch with a known-breaking change under a minor or patch number and confirm the job goes red
- [ ] 2.4 Dispatch with the matching major number and confirm the job goes green
- [ ] 2.5 Measure the real job duration and set `timeout-minutes` from it

## 3. Tuning knobs (apply only if a measured run needs them)

- [ ] 3.1 If the build dominates the budget, add a `Swatinem/rust-cache` step (the runner is cold on every run)
- [ ] 3.2 Add targeted `--features` for public API behind non-default features, once the `--exclude` set is settled
- [ ] 3.3 If full-workspace rustdoc stays too slow, scope the check with a `[package.metadata.semver-checks]` allow-list on the published library crates

## 4. Documentation

- [ ] 4.1 Note in the crate-release runbook that a breaking change requires a major version bump, and that the dry-run gate enforces it
- [ ] 4.2 Note the gate's blind spot: behavioural breaks behind an unchanged signature are not caught

## 5. Out of scope

- Remediation of the already-published `1.22.x` crates. Tracked separately as a one-off release decision.
- A per-pull-request semver canary. Rejected as too noisy, because mid-cycle the manifest version equals the last release, so every breaking pull request would read as an insufficient bump.
- Pinning intra-workspace dependencies with `=`. Not needed once version numbers are honest, since caret is the correct range for honest-semver crates.
- Making the gate unskippable by moving it into `ci-crates-version-bump.yml`. Deferred.
