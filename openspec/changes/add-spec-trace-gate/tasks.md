The PR of `max/mixnet-core-specs` holds sections 1, 2, 3 and 6. Sections 4, 5 and 7 land in later PRs.

## 1. Check what the gate fails on

- [x] 1.1 With Tracey 1.4.0, on a scratch branch: add a marker that names an unknown id, and a requirement with no marker. Run `tracey query validate --deny warnings`. Record which of the two fails the run, in `design.md` T2. Also check what version Tracey gives a marker with no `+N`: `tracey.mts` reads it as version 1, so a marker with no version fails after the spec moves to `+2`. (Run by a person: Tracey is not in the sandbox, and on 2026-11-09 the sandbox could not download the release.) Done 2026-11-10: the uncovered requirements pass, the unknown id fails, and Tracey reads a marker with no version as version 1. Results in `design.md` T2.
- [x] 1.2 If an uncovered requirement fails the run and no Tracey setting separates the two, write a reader of our own as the gate. Not needed: task 1.1 shows that Tracey separates the two.

## 2. The markers as the only code links

- [x] 2.1 `coverage.mts` takes the code and the tests of each requirement with a Tracey id from `tracey query --json rule <id>`. Done 2026-11-10. The spike `tracey.mts`, which read the markers a second time, is deleted.
- [x] 2.2 Remove the code entries from `openspec/changes/add-mixnet-core-specs/edges.proposed.json`. Keep the threat-model entries. Done 2026-11-09: 64 edges to 14.
- [x] 2.3 The records check fails on a hand-written `realises` edge from code to a requirement in the scope. Done 2026-11-09. The trial copy shows the error for an old hand edge.

## 3. The config and the gate

- [x] 3.1 `.config/tracey/config.styx`: the three core specs with the code paths of T3, including `gateway/**`. Done 2026-11-09. SDK and Network Monitor V3 paths join in 4.2 and 5.3.
- [x] 3.2 `.github/workflows/ci-spec-trace.yml`, as in T6, with Tracey 1.4.0 and its SHA-256. Written 2026-11-10. The workflow has not run on GitHub yet: the first PR run checks it.
- [x] 3.3 The canary of T8: the CI job plants an unknown id in its checkout and expects Tracey to fail and name it. Done 2026-11-10. Task 1.1 run 2 is the same check by hand.
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

## 6. The check at commit time

- [x] 6.1 A lefthook pre-commit job runs `tracey query validate --deny warnings` on staged Rust files. It always exits 0, and it prints one line and exits 0 when `tracey` is missing. Done 2026-11-10.

## 7. When add-mixnet-core-specs is archived

- [ ] 7.1 In the archive commit, change the three config paths to `openspec/specs/<capability>/spec.md` (T4).
- [ ] 7.2 Write the `## Purpose` of `sphinx-packet-format`, `client-cover-traffic` and `mix-node-delay`. Run `openspec validate --specs --strict`.
