# Design: Semver Release Gate

## Decisions

### Keep lockstep versioning and caret dependencies

We considered three fixes for the integrator's breakage.

| Option | Change | Ongoing cost | Chosen |
|---|---|---|---|
| Honest semver, enforced | Break = next workspace major; keep caret deps | A per-release check, automated here | Yes |
| `=` sibling pins | Change intra-workspace deps to `version = "=X.Y.Z"`; keep shipping lockstep minors | None | No |
| Per-crate versioning | Decouple the workspace version so only broken crates bump | Large one-time refactor | No |

The `=` pins are a workaround for dishonest version numbers. Once the gate makes numbers honest, caret is the correct range and the pins are redundant. Per-crate versioning is the precise end state but a much larger change to the release process; it stays on the table for later.

### Gate in the dry-run, not per pull request

The release dry-run (`ci-crates-publish-dry-run.yml`) already applies the target version locally via `cargo workspaces version custom`, without committing. So the dry-run is a true gate: `cargo-semver-checks` reads the applied version and validates it, and failing the dry-run costs nothing because no PR or publish follows.

A per-pull-request canary was rejected. Mid-cycle the manifest version still equals the last release, so every breaking pull request would read as an insufficient bump and the check would be red on half of them. Between majors we break APIs on purpose, so a per-PR gate fights the normal workflow.

### Scope to the nym-sdk dependency closure

`cargo-semver-checks` inspects each crate independently. The integrator's breakage was in `nym-crypto`, `nym-topology`, and other transitive crates, not in `nym-sdk`'s own API, so a check scoped to `nym-sdk` alone would have missed every one. The first implementation used `--workspace`, but at roughly 100 to 230 seconds of rustdoc build per crate, the full published set runs for hours and does not fit the job budget. It also checks crates (`node-status-api`, contract-build crates) that are published only by lockstep and have no external library consumers.

The scope is therefore the `nym-sdk` published dependency closure: `cargo tree -p nym-sdk --all-features --edges normal --target all`, filtered to workspace `nym-*` crates. The `--target all` keeps wasm-only edges, such as `nym-wasm-utils` through `nym-client-core`, so a wasm consumer is not missed. This is exactly the set a library consumer of `nym-sdk` can be broken by, including every transitive crate the integrator had to pin. It is computed from the lockfile at run time so it tracks the dep tree rather than rotting as a hardcoded list. `publish = false` crates in the closure are skipped by the tool. A closure crate with no crates.io baseline (a new unreleased dependency) is a hard error in the tool, not a skip, so the step checks the sparse index per crate and drops any that is not published yet. That crate rejoins the gate on its first release with no manual change, so there is no hardcoded exclude list to prune.

## Trade-offs and limits

- The gate fires only when the dry-run is dispatched, so it is enforced by process, not by branch protection. Moving the same steps into `ci-crates-version-bump.yml` would make it unskippable; deferred until the dry-run gate proves itself.
- `cargo-semver-checks` catches public-API breaks only. A behavioural change behind an unchanged signature passes the gate, so release judgement is reduced, not removed.
- Building rustdoc for each crate plus its baseline is heavy (100 to 230 seconds per crate, measured). Scoping to the `nym-sdk` closure cuts the crate count, and a `Swatinem/rust-cache` step persists `target/` so only the first cold run pays the full build cost; reruns are warm, and `cargo-semver-checks` also reuses its baseline rustdoc between runs. The job sets no `timeout-minutes`, so it uses GitHub's default 360-minute cap; a cold run over the full closure can take hours, and a genuine hang is cancelled by hand rather than guarded by a tuned limit. The tool is installed from source (`cargo install cargo-semver-checks`), matching the existing `cargo install cargo-workspaces` step; no version pin.
- The tool's default feature heuristic enables almost every feature at once, which can fail to compile crates with wasm-only or mutually exclusive features, because the default set is then an invalid combination. The gate uses the default for maximum API coverage. When a crate fails to build on the default, a pinned valid `--features` set can restore the build, and is preferred over `--exclude` because it keeps that crate's API in the gate. `--exclude` is the last resort, for a crate that compiles under no feature set the native runner supports. Both are decided in calibration.
- A publishable crate with no prior crates.io release is a hard error in the tool, not a skip, and wasm-only crates will not build on the native runner. Both must go into `--exclude`. The exact set is unknown until a first calibration run, which also fixes the feature flags and the timeout. This calibration gates relying on the check.

## Baseline source

The default baseline is the greatest version on crates.io for each crate. All published crates have a prior release, so no git tag is needed. The crate version line (1.21.6, 1.22.2) is numbered separately from the binary tag line (`v1.1.x`), and there is no git tag for a crate version, which is why a crates.io baseline is simpler than a `--baseline-rev`.
