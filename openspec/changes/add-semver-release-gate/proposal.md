# Add Semver Release Gate

## Why

An integrator pinned `nym-sdk = "1.21.6"` and still got a broken build. Cargo did not hold the other `nym-*` crates at 1.21.6; it pulled newer `1.22.x` versions of them, and those `1.22.x` releases carried breaking API changes. The integrator had to hand-pin about ten `nym-*` crates to `=1.21.6` to force a matching set.

Two faults combined. We ship breaking changes under minor bumps: the whole workspace shares one version (`Cargo.toml` `version = "1.22.2"`, inherited by ~150 crates via `cargo workspaces`), and a `1.21 -> 1.22` step is a minor bump, which semver says must stay compatible. Our intra-workspace dependencies also use caret ranges (`version = "1.22.2"` is `^1.22.2`), so a pinned `nym-sdk` does not pin its siblings, and Cargo climbs to the newest release in range.

We keep lockstep versioning and keep caret dependencies. We add a check that makes lockstep honest: at release time it fails when the chosen version number is too small for the public-API change. With honest version numbers, caret ranges are correct again, so no dependency pinning is needed.

## What Changes

- The crate-publishing dry-run (`ci-crates-publish-dry-run.yml`) runs `cargo-semver-checks` after the local version bump. The bump has already written the target version into every manifest, so the check compares each published crate's public API against its latest crates.io release and fails when the target version is an insufficient bump.
- The check spans every published crate (`--workspace`), not just `nym-sdk`. The integrator's breakage was in transitive crates such as `nym-crypto` and `nym-topology`, which `cargo-semver-checks` only detects when it inspects those crates directly. `publish = false` crates are skipped. A publishable crate with no prior crates.io release is a hard error, not a skip, so a release that adds a new crate must exclude it for its first release.
- The gate lives only in the manual dry-run, the release rehearsal step, so there is no per-pull-request cost. A too-small version number goes red before the real version bump or publish runs.
- The dry-run job timeout rises from 35 to 90 minutes, because the check builds rustdoc for every published crate plus its baseline.

## Capabilities

### New Capabilities

- `crate-release`: the crate-publishing release process gains a semver gate in the dry-run that validates the chosen workspace version against the published public API.

### Modified Capabilities

<!-- none -->

## Impact

- `.github/workflows/ci-crates-publish-dry-run.yml`: adds a `cargo-semver-checks` install step and a semver gate step after the local version bump; raises the job timeout to 90 minutes.
- No source or wire-protocol change. This is release-process tooling only.
- The gate catches public-API breaks (removed or changed items, signatures, trait changes). It does not catch behavioural breaks behind an unchanged signature, so it reduces the release judgement call rather than removing it.
- The gate needs one calibration run before it is relied on: the first real dry-run reveals the crates to add to `--exclude` (wasm-only crates that do not build on the native runner, and any newly added crate), and whether the `timeout-minutes` value holds.
- Out of scope, tracked separately: remediation of the already-published `1.22.x` crates that shipped breaking changes under a minor number. That is a one-off release decision, not a standing capability.
- Out of scope: making the gate unskippable. It fires only when the dry-run is dispatched. If required later, the identical steps drop into `ci-crates-version-bump.yml`.
