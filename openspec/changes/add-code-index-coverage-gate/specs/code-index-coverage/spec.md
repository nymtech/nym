# code-index-coverage Spec Delta

## ADDED Requirements

### Requirement: Verified roots are derived from the canonical roots list

The set of source roots verified for citability SHALL be derived from the canonical roots list (`ROOTS` in `documentation/indexed-sources.mjs`), not from a separately maintained list. A static check SHALL assert that every configured root is covered by at least one coverage probe, where a probe covers a root when the probe's path fragment is equal to or nested under that root. The check SHALL fail, naming the root, when a configured root has no covering probe. This check SHALL run without a deployment (as a unit test), so drift is caught before merge.

#### Scenario: A newly added root without a probe

- **WHEN** a root is added to the canonical roots list but no coverage probe covers it
- **THEN** the static completeness check fails and names that root

#### Scenario: Every root has a covering probe

- **WHEN** every configured root is covered by at least one probe, including probes whose path fragment is a subpath of the root
- **THEN** the static completeness check passes

### Requirement: Live citability probe, skip-not-fail for absent roots

For each configured root present in the checkout under test, the deployment check SHALL issue a `search_code` query and assert a returned hit whose path is under that root. A root not present in the checkout SHALL be skipped rather than failed, since the deployed index is built from whichever branch it was built from. A root present in the checkout whose probe returns no hit under it SHALL fail the check.

#### Scenario: Root present but uncited

- **WHEN** a configured root exists in the checkout and its probe query returns no hit whose path is under that root
- **THEN** the coverage check fails for that root

#### Scenario: Root absent from the checkout

- **WHEN** a configured root does not exist in the checkout under test
- **THEN** its probe is skipped and does not fail the run

### Requirement: Probe data excluded from the served bundle

The coverage probe data SHALL live outside any module traced into the MCP server bundle, so probe queries are neither shipped to nor parsed by the production server.

#### Scenario: Probes are not in the lambda trace

- **WHEN** the docs deploy artefact is built
- **THEN** the coverage probe data file is not among the files traced into the `/api/mcp` lambda

### Requirement: Coverage is repo-aware

When the index draws from more than one producer repo, the verified-roots set SHALL include every producer repo's roots, and completeness SHALL be asserted per repo, so no repo's citability is silently omitted.

#### Scenario: A second repo with no probes

- **WHEN** a second producer repo's roots are configured but none of them is covered by a probe
- **THEN** the completeness check fails rather than passing with that repo unverified
