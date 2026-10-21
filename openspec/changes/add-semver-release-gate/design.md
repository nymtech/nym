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

### Check all published crates

`cargo-semver-checks` inspects each crate independently. The integrator's breakage was in `nym-crypto`, `nym-topology`, and other transitive crates, not in `nym-sdk`'s own API, so a check scoped to `nym-sdk` would have missed every one. `--workspace` covers the published set and skips `publish = false` and unpublished crates on its own.

## Trade-offs and limits

- The gate fires only when the dry-run is dispatched, so it is enforced by process, not by branch protection. Moving the same steps into `ci-crates-version-bump.yml` would make it unskippable; deferred until the dry-run gate proves itself.
- `cargo-semver-checks` catches public-API breaks only. A behavioural change behind an unchanged signature passes the gate, so release judgement is reduced, not removed.
- Building rustdoc for every published crate plus its baseline is heavy. The job timeout rises to 90 minutes as a provisional value, to be tuned once a real run is measured. The runner has no build cache, so every run is cold; add `Swatinem/rust-cache` if the build dominates. The tool is installed from source (`cargo install cargo-semver-checks`), matching the existing `cargo install cargo-workspaces` step; no version pin.
- The tool's default feature heuristic enables almost every feature at once, which can fail to compile crates with wasm-only or mutually exclusive features. The gate uses the default, for maximum API coverage. Compilation is monotonic, so narrowing the feature set (`--only-explicit-features`) cannot fix a build the full set breaks on; it only loses coverage. So the first run uses the default, and any crate that fails to build is handled in calibration by `--exclude` or a pinned `--features`.
- A publishable crate with no prior crates.io release is a hard error in the tool, not a skip, and wasm-only crates will not build on the native runner. Both must go into `--exclude`. The exact set is unknown until a first calibration run, which also fixes the feature flags and the timeout. This calibration gates relying on the check.

## Baseline source

The default baseline is the greatest version on crates.io for each crate. All published crates have a prior release, so no git tag is needed. The crate version line (1.21.6, 1.22.2) is numbered separately from the binary tag line (`v1.1.x`), and there is no git tag for a crate version, which is why a crates.io baseline is simpler than a `--baseline-rev`.
