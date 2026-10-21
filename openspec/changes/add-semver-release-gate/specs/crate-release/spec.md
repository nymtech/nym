# crate-release Spec Delta

## ADDED Requirements

### Requirement: The release dry-run gates on semver compatibility

The crate-publishing dry-run SHALL run a semver check after it applies the target version to the workspace manifests. The check SHALL compare each published crate's public API against that crate's latest release on crates.io, and SHALL fail the dry-run when the target version is too small a bump for the observed API change. `publish = false` crates SHALL be skipped. A publishable crate with no prior crates.io release is a hard error in the tool, so a release that adds a new crate SHALL exclude that crate for its first release.

#### Scenario: A breaking change under a minor bump fails the dry-run

- **WHEN** the dry-run is dispatched with a target version that is a minor or patch bump over the last release, and a published crate's public API has a breaking change
- **THEN** the semver check fails and the dry-run goes red before the publish dry-run runs

#### Scenario: A breaking change under a matching major bump passes

- **WHEN** the dry-run is dispatched with a major version bump over the last release, and the public-API changes are covered by that major bump
- **THEN** the semver check passes

#### Scenario: A compatible change under a minor bump passes

- **WHEN** the dry-run is dispatched with a minor bump and no published crate has a breaking API change
- **THEN** the semver check passes

### Requirement: The gate spans the nym-sdk dependency closure, not only the SDK entry point

The semver check SHALL inspect every crate in the `nym-sdk` published dependency closure, not only `nym-sdk` itself, because a breaking change in a transitive crate breaks consumers even when the entry point's own API is unchanged. The closure SHALL be computed from the lockfile so it tracks the dependency tree. Crates outside the closure, published only by lockstep with no external library consumers, SHALL NOT be checked. Under lockstep versioning the single workspace version SHALL cover the largest break anywhere in the closure.

#### Scenario: A break in a transitive crate fails the gate

- **WHEN** a transitive crate in the closure (for example the crypto or topology crate) has a breaking API change, while the SDK crate's own public API is unchanged, and the target version is only a minor bump
- **THEN** the semver check fails, because the transitive crate needs a major bump and lockstep forces the workspace version to match

### Requirement: The gate runs at release rehearsal, not per pull request

The semver check SHALL run in the manually dispatched release dry-run, not on every pull request, so that routine pull requests carry no semver-check cost. The check SHALL run with the target version already written into the manifests, so that it validates the chosen version rather than only detecting that an API changed.

#### Scenario: Routine pull requests are not gated

- **WHEN** a pull request changes crate source but no release dry-run is dispatched
- **THEN** no semver check runs against that pull request
