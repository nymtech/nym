## Context

`add-mixnet-core-specs` adds 21 requirements with Tracey ids and 60 markers in the Rust code. Tracey 1.4.0 reads them and reports 20 of 21 requirements with code and 5 of 21 with a test. `documentation/docs/lib/snippets/tracey.mts` reads the same markers into docs records. Nothing runs either reader in CI, and the hand-written code edges in `edges.proposed.json` are a second copy of the same links. On 2026-11-09, `tracey.mts --compare` found 48 of the 50 hand edges in the markers. The other 2 name `SurbAck::construct`, which develop renamed to `construct_legacy`.

## Decisions

### T1: The markers are the only source of code-to-requirement links

A hand edge names its code by `<path>#<symbol>`. That is a lookup, so a rename breaks it with no error until something resolves it. A marker sits on the line above the code, so it moves with the code. `tracey.mts` already turns `impl` markers into `realises` edges and `verify` markers into witnesses, and it resolves each one to an item with the extractor's matcher (`items()` and `matchSymbol()` in `lib/retrieval/items.mjs`).

So `records.mts` and `coverage.mts` read the code edges from `tracey.mts`. A `realises` edge from code to a requirement in the scope, written by hand in `.nym-codex/edges.json` or `edges.proposed.json`, is an error in the records check. The edges from a requirement to the threat model are not in the code. They stay hand-written, and the threat-model owner reviews them.

### T2: The gate fails on a broken reference, and reports a gap

The blocking job fails on three faults:

| Fault | Example |
|---|---|
| A marker names an id that no spec in the config defines | `// r[impl sphinx.size-clases]` |
| Two requirements have the same id | `r[delay.node-holds]` in two spec files |
| A marker names an old version of a requirement | `// r[impl sphinx.route]` after the spec has `r[sphinx.route+2]` |

The job reports, and does not fail on, a requirement with no `impl` marker and a requirement with no `verify` marker. This lets the ids of a spec land before its markers, so the SDK and Network Monitor V3 work can go in steps.

**Open fact.** Tracey is not in the sandbox where this change was written, so nobody has checked whether `tracey query validate --deny warnings` treats an uncovered requirement as a warning. Task 1.1 checks it with real Tracey. If the flag fails on an uncovered requirement, and Tracey has no setting to separate the two, the three checks above move into `tracey.mts`, which already parses the markers and the spec ids. Tracey then stays for the dashboard, the editor and the agent, and is not part of the gate.

### T3: The scope is a list of specs, not of code paths

The scope is 7 specs. For each spec, the config lists every code path that holds a marker for it:

| Spec | Code paths |
|---|---|
| `sphinx-packet-format` | `common/nymsphinx/**`, `common/topology/**`, `common/client-libs/mixnet-client/**`, `nym-node/**` |
| `client-cover-traffic` | `common/client-core/**`, `common/nymsphinx/**` |
| `mix-node-delay` | `common/nymsphinx/**`, `common/client-core/**`, `nym-node/**`, `gateway/**`, `nym-registration-client/**` |
| `sdk-mixnet-stream` | `sdk/rust/nym-sdk/**` |
| `nym-network-monitor` | `nym-network-monitor-v3/**` |
| `network-monitor-config-score` | `nym-network-monitor-v3/nym-network-monitor-orchestrator/**` |
| `network-monitor-epoch-performance` | `nym-network-monitor-v3/nym-network-monitor-orchestrator/**` |

A cut by code path loses true links. For example, without `nym-node/**`, the replay requirement and its witness would show as uncovered. A spec cannot leave the config while markers name its ids, because each such marker then names an unknown id and the gate fails. With these paths, all 20 core requirements that have code in this repository keep their markers, and all 5 witnesses count. `delay.only-hop-reads` stays uncovered, because its code is in the `sphinx-packet` crate.

Out of the scope: `network-monitor` (15 requirements), which is the older monitor inside `nym-api`, and `sdk-service-provider-example` (11), which describes examples and docs.

### T4: The spec paths before and after the archive of add-mixnet-core-specs

Until `add-mixnet-core-specs` is archived, the config reads its three specs from `openspec/changes/add-mixnet-core-specs/specs/**`. `openspec archive` moves them to `openspec/specs/<capability>/spec.md`. A check on a scratch copy on 2026-11-09 showed that the archive keeps all 21 `r[...]` lines. The archive commit therefore also:

- changes the three config paths to `openspec/specs/sphinx-packet-format/spec.md`, `openspec/specs/client-cover-traffic/spec.md` and `openspec/specs/mix-node-delay/spec.md`.
- writes the `## Purpose` of the three new specs. `openspec archive` writes a placeholder, and `openspec validate --specs --strict` fails on it. 17 specs on develop already fail for this reason.

The CI job watches both paths, so it runs on the archive commit.

### T5: Ids in the archived specs are id lines, not MODIFIED deltas

The SDK and Network Monitor V3 specs are already in `openspec/specs/`. The OpenSpec way to change them is a MODIFIED delta, which copies each requirement in full: 61 requirements, with the risk that a copy differs from its source. An id line does not change what a requirement says. So this change edits the four archived specs directly, and adds only one id line for each requirement, in the form that `add-mixnet-core-specs` uses: the `### Requirement:` heading, an empty line, `r[<id>]`, then the text. A check on 2026-11-09 showed that `openspec validate --specs --strict` accepts an id line in an archived spec.

The ids use a short prefix for each spec: `sdk.` for `sdk-mixnet-stream`, `nmv3.` for `nym-network-monitor`, `nmv3.score.` for `network-monitor-config-score`, and `nmv3.epoch.` for `network-monitor-epoch-performance`. The part after the prefix is a short name of the requirement, as in `sdk.open-ack`.

### T6: The CI job

A new workflow, `.github/workflows/ci-spec-trace.yml`, and not a step in `ci-codex-snippets`, for two reasons. The codex job does not run on `openspec/changes/**` or `.config/tracey/**`. And the gate must not wait for a docs dependency install.

| Part | Value |
|---|---|
| Trigger | `pull_request` on `openspec/specs/**`, `openspec/changes/add-mixnet-core-specs/**`, `.config/tracey/**`, `**/*.rs`, and the workflow file |
| Tool | Tracey 1.4.0, `tracey-x86_64-unknown-linux-gnu.tar.xz` from the GitHub release, with its SHA-256 written in the workflow. `cargo install tracey` (1.3.0) does not build without `--locked`, so the job does not use it |
| Command | `tracey query validate --deny warnings`, or the `tracey.mts` checks if task 1.1 finds the fallback necessary |
| Blocking | A required status check on `develop`, which a repository admin sets |

### T7: The advisory note at commit time

A lefthook pre-commit job on staged `*.rs` files runs `node documentation/docs/lib/snippets/tracey.mts --changed`. The new `--changed` mode reads the staged diff, finds each item whose lines changed and that carries a marker, and prints the id, the verb and the first sentence of the requirement:

```
sphinx.ack-packet (impl): Each data packet carries an acknowledgement packet for its fragment
  changed: common/nymsphinx/acknowledgements/src/surb_ack.rs SurbAck::construct_legacy
```

The job always exits 0. If `node` is missing, it prints one line and exits 0, in the same way as the `rust-lint` job does with `cargo`. The note asks the question. A person or an agent answers it.

### T8: The gate has its own canary

A check that cannot fail is a constant (`add-codex-assurance`, A2). A test fixture holds one spec and one source file with a marker that names an unknown id, and a requirement with no marker. The test runs the gate command on the fixture. It expects a failure that names the bad marker, and a report, not a failure, for the uncovered requirement. If the gate is in `tracey.mts` (T2), `tracey.test.mts` holds this test. If the gate is Tracey, the CI job runs the fixture as a first step and fails when Tracey passes it.

## Risks

| Risk | Answer |
|---|---|
| The Tracey release goes away or changes | The SHA-256 in the workflow stops a changed file. The `tracey.mts` checks (T2) can replace the tool |
| A requirement id changes, and many markers then name an unknown id | Tracey's version suffix (`+2`) changes the text and keeps the id. A real rename changes the markers in the same commit, and the gate shows each one |
| The markers add noise to the code | One comment line above each item. The 60 markers of the core specs are the measure: if the code owners reject them, this change stops before the SDK and Network Monitor V3 steps |
| The advisory note is ignored | It is a prompt, not a guard. The witness tests guard the facts that matter |
