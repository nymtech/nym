## 1. Check what the gate fails on

- [ ] 1.1 With Tracey 1.4.0, on a scratch branch: add a marker that names an unknown id, and a requirement with no marker. Run `tracey query validate --deny warnings`. Record which of the two fails the run, in `design.md` T2. (Run by a person: Tracey is not in the sandbox.)
- [ ] 1.2 If an uncovered requirement fails the run and no Tracey setting separates the two, put the three checks of T2 into `tracey.mts` and make that the gate command.

## 2. The markers as the only code links

- [ ] 2.1 `records.mts` and `coverage.mts` read the `realises` code edges and the witnesses from `tracey.mts`.
- [ ] 2.2 Remove the code entries from `openspec/changes/add-mixnet-core-specs/edges.proposed.json`. Keep the threat-model entries.
- [ ] 2.3 The records check fails on a hand-written `realises` edge from code to a requirement in the scope.
- [ ] 2.4 Tests: a marker above a renamed item still gives its edge; a marker that names an unknown id is an error.

## 3. The config and the gate

- [ ] 3.1 `.config/tracey/config.styx`: the three core specs with the code paths of T3, including `gateway/**`.
- [ ] 3.2 `.github/workflows/ci-spec-trace.yml`, as in T6, with Tracey 1.4.0 and its SHA-256.
- [ ] 3.3 The canary of T8: a fixture with an unknown id and an uncovered requirement, and a test that expects a failure for the first and a report for the second.
- [ ] 3.4 Ask a repository admin to make `ci-spec-trace` a required check on `develop`.

## 4. The SDK spec

- [ ] 4.1 Add an `r[sdk.<name>]` line to each of the 12 requirements in `openspec/specs/sdk-mixnet-stream/spec.md`. Run `openspec validate --specs --strict`.
- [ ] 4.2 Add the spec to the config with `sdk/rust/nym-sdk/**`.
- [ ] 4.3 Add `impl` markers in `sdk/rust/nym-sdk`, and `verify` markers on the tests that check a requirement.

## 5. The Network Monitor V3 specs

- [ ] 5.1 Add id lines to `nym-network-monitor` (29), `network-monitor-config-score` (9) and `network-monitor-epoch-performance` (11), with the prefixes of T5.
- [ ] 5.2 Write the `## Purpose` of `network-monitor-config-score` and `network-monitor-epoch-performance`, which are still placeholders and fail strict validation.
- [ ] 5.3 Add the three specs to the config with the paths of T3.
- [ ] 5.4 Add `impl` and `verify` markers in `nym-network-monitor-v3`.

## 6. The advisory note

- [ ] 6.1 `tracey.mts --changed`: map the staged diff to the marked items, and print the id, the verb and the first sentence of each requirement, as in T7.
- [ ] 6.2 Add the lefthook pre-commit job. It always exits 0, and it prints one line and exits 0 when `node` is missing.
- [ ] 6.3 Test: a staged change to a marked item prints its requirement; a change to an item with no marker prints nothing.

## 7. When add-mixnet-core-specs is archived

- [ ] 7.1 In the archive commit, change the three config paths to `openspec/specs/<capability>/spec.md` (T4).
- [ ] 7.2 Write the `## Purpose` of `sphinx-packet-format`, `client-cover-traffic` and `mix-node-delay`. Run `openspec validate --specs --strict`.
