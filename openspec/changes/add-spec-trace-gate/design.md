## Context

`add-mixnet-core-specs` adds 21 requirements with Tracey ids and markers in the Rust code. When this change started, on 2026-11-09, Tracey 1.4.0 reported 20 of 21 requirements with code and 5 of 21 with a test. Nothing ran Tracey in CI, and the hand-written code edges in `edges.proposed.json` are a second copy of the same links. On 2026-11-09, a comparison script on the branch found 48 of the 50 hand edges in the markers. The other 2 name `SurbAck::construct`, which develop renamed to `construct_legacy`. On 2026-11-10, `add-mixnet-core-specs` closed the gaps: each of the 21 requirements now has an `impl` marker and a `verify` marker.

## Decisions

### T1: The markers are the only source of code-to-requirement links

A hand edge names its code by `<path>#<symbol>`. That is a lookup, so a rename breaks it with no error until something resolves it. A marker sits on the line above the code, so it moves with the code.

So Tracey owns the code links, and the repository has no second reader of the markers. `coverage.mts` asks Tracey for the markers of each requirement with a Tracey id (`tracey query --json rule <id>`), and counts a requirement with an `impl` marker as realised and one with a `verify` marker as tested. The docs records hold no code edges for these requirements. A `realises` edge from code to a requirement with a Tracey id, written by hand in `.nym-codex/edges.json`, is an error in the records check. The records check reads only the archived specs in `openspec/specs/`, so this rule applies to a spec from the commit that archives its change. Until then, the `edges.proposed.json` of the change holds no code edges. The edges from a requirement to the threat model are not in the code. They stay hand-written, and the threat-model owner reviews them.

### T2: The gate fails on a broken reference and on a gap

The blocking job fails on three broken references:

| Fault | Example |
|---|---|
| A marker names an id that no spec in the config defines | `// r[impl sphinx.size-clases]` |
| Two requirements have the same id | `r[delay.node-holds]` in two spec files |
| A marker names an old version of a requirement | `// r[impl sphinx.route]` after the spec has `r[sphinx.route+2]` |

Only the spec files and the code paths that the Tracey config names are read. A copy of a spec in an archived change, or in a MODIFIED delta of an open change, is not read, so it cannot define an id twice. A comment of the marker form outside the code paths is not read either.

The job also fails on two kinds of gap, and lists each one in the job summary: a requirement with no `impl` marker or no `verify` marker, and a requirement block with no `r[id]` line in a spec that has `r[id]` lines. Tracey cannot see a requirement with no id, so `tools/internal/spec-trace-gaps.sh` finds it in the spec text: an `r[id]` line must come after the `### Requirement:` heading and before the first scenario. A gap is a fault to fix, not a report to read. A feature branch can fail this job for as long as its gaps are open, so the ids of a spec can still land before its markers there. The merge to `develop` waits until each gap is closed. No requirement is exempt. Most of the code of `delay.only-hop-reads` is in the `sphinx-packet` crate, so its `impl` marker is on `wrap_processed_sphinx_packet`, the call in this repository, and a 4-hop test in this repository is its witness.

**Checked with Tracey 1.4.0 on 2026-11-10 (task 1.1).** `validate` itself does not fail on a gap, so the gaps step reads `tracey query --json uncovered` and `untested`:

| Run | `tracey query validate --deny warnings` |
|---|---|
| The tree on that date: 1 requirement with no `impl` marker, 16 with no `verify` marker | Exit 0, "No validation errors found" |
| A planted `// r[impl sphinx.size-clases]` | Exit 1, "1 unknown rule reference(s)" |
| The spec moves to `r[sphinx.route+2]`, and the 3 markers have no version | Exit 1, 3 `StaleRequirement` errors. `tracey query stale` lists the same 3 markers |
| `r[delay.node-holds]` in a second spec | Prints `CONFIG ERROR ... Duplicate requirement 'delay.node-holds'`, then validates the old state and exits 0. The CI step fails when the output holds `CONFIG ERROR` |
| `// r[impl sphinx.route]` after the last item of a file | Exit 0 |

So `validate` does not fail on an uncovered requirement, and Tracey reads a marker with no version as version 1. A marker with no item below it passes. Nothing in this change catches that, and it is a cost of having one reader. The gate is Tracey (decided 2026-11-10).

### T3: The scope is a list of specs, not of code paths

The scope is 7 specs. The config holds them in one group with one list of code paths: each path below. A marker in one of these paths can name an id of each spec in the group. The group holds the three core specs now. Tasks 4 and 5 add the other four with their paths.

| Spec | Code paths |
|---|---|
| `sphinx-packet-format` | `common/nymsphinx/**`, `common/topology/**`, `common/client-libs/mixnet-client/**`, `nym-node/**` |
| `client-cover-traffic` | `common/client-core/**`, `common/nymsphinx/**` |
| `mix-node-delay` | `common/nymsphinx/**`, `common/client-core/**`, `nym-node/**`, `gateway/**`, `nym-registration-client/**` |
| `sdk-mixnet-stream` | `sdk/rust/nym-sdk/**` |
| `nym-network-monitor` | `nym-network-monitor-v3/**` |
| `network-monitor-config-score` | `nym-network-monitor-v3/nym-network-monitor-orchestrator/**` |
| `network-monitor-epoch-performance` | `nym-network-monitor-v3/nym-network-monitor-orchestrator/**` |

A cut by code path loses true links. For example, without `nym-node/**`, the replay requirement and its witness would show as uncovered. A spec cannot leave the config while markers name its ids, because each such marker then names an unknown id and the gate fails. With these paths, all 21 core requirements keep their markers, and all their witnesses count.

Out of the scope: `network-monitor` (15 requirements), which is the older monitor inside `nym-api`, and `sdk-service-provider-example` (11), which describes examples and docs.

### T4: The spec paths before and after the archive of add-mixnet-core-specs

Until `add-mixnet-core-specs` is archived, the config reads its three specs from `openspec/changes/add-mixnet-core-specs/specs/**`. `openspec archive` moves them to `openspec/specs/<capability>/spec.md`. A check on a scratch copy on 2026-11-09 showed that the archive keeps all 21 `r[...]` lines. The archive commit therefore also:

- changes the three config paths to `openspec/specs/sphinx-packet-format/spec.md`, `openspec/specs/client-cover-traffic/spec.md` and `openspec/specs/mix-node-delay/spec.md`.
- writes the `## Purpose` of the three new specs. `openspec archive` writes a placeholder, and `openspec validate --specs --strict` fails on it. 17 specs on develop already fail for this reason.

The CI job runs on each pull request, so it runs on the archive commit.

### T5: Ids in the archived specs are id lines, not MODIFIED deltas

The SDK and Network Monitor V3 specs are already in `openspec/specs/`. The OpenSpec way to change them is a MODIFIED delta, which copies each requirement in full: 61 requirements, with the risk that a copy differs from its source. An id line does not change what a requirement says. So this change edits the four archived specs directly, and adds only one id line for each requirement, in the form that `add-mixnet-core-specs` uses: the `### Requirement:` heading, an empty line, `r[<id>]`, then the text. A check on 2026-11-09 showed that `openspec validate --specs --strict` accepts an id line in an archived spec.

The ids use a short prefix for each spec: `sdk.` for `sdk-mixnet-stream`, `nmv3.` for `nym-network-monitor`, `nmv3.score.` for `network-monitor-config-score`, and `nmv3.epoch.` for `network-monitor-epoch-performance`. The part after the prefix is a short name of the requirement, as in `sdk.open-ack`.

### T6: The CI job

A new workflow, `.github/workflows/ci-spec-trace.yml`, and not a step in `ci-codex-snippets`, for two reasons. The codex job has a path filter, so it cannot be a required check. And the gate must not wait for a docs dependency install.

| Part | Value |
|---|---|
| Trigger | Each `pull_request`, with no path filter. A workflow that a path filter skips sends no status, so a required check on it stays pending and the PR cannot merge. The job is a download and a scan of the code paths, so it is short |
| Tool | Tracey 1.4.0, `tracey-x86_64-unknown-linux-gnu.tar.xz` from the GitHub release, with its SHA-256 written in the workflow. `cargo install tracey` (1.3.0) does not build without `--locked`, so the job does not use it |
| Command | `tracey query validate --deny warnings`, which fails on an unknown id or an old version, and a `grep` that fails on `CONFIG ERROR`. Then `tools/internal/spec-trace-gaps.sh` fails on each gap of T2 |
| Blocking | A required status check on `develop`, which a repository admin sets |

### T7: The advisory note at commit time

A lefthook pre-commit job on staged `*.rs` and `*.md` files runs `tracey query validate --deny warnings` and `tools/internal/spec-trace-gaps.sh`, so a developer sees a broken reference or a gap before CI does. A Tracey daemon that is already running can miss a file that is new, so the job can show a gap one commit late. The job always exits 0. If `tracey` is missing, it prints one line and exits 0, in the same way as the `rust-lint` job does with `cargo`.

To see which requirement a piece of code names while you edit it, use `tracey lsp` in the editor. An agent can use `tracey ai`. After a spec moves to `+2`, `tracey query stale` lists each marker whose code a person must check against the new text.

### T8: The gate has its own canary

A check that cannot fail is a constant (`add-codex-assurance`, A2). The CI job writes `// r[impl canary.unknown]` into `common/nymsphinx/src/` in its own checkout, before the real run. It also writes a spec file with two requirements: `r[canary.gap]`, which no marker names, and one with no `r[id]` line. Then it removes the files and stops the Tracey daemon (`tracey kill`), because a running daemon does not see a new file at once. The first CI run showed this: with the canary after the gate, the daemon of the gate passed the planted id. The job fails unless Tracey then fails and names `canary.unknown`, and the gaps script fails and names both planted requirements. The gaps step runs the same script, so the canary checks its `jq` paths too. This also checks that the real config scans the code paths and the spec files. The SHA-256 pins the binary, so the canary has no other Tracey behaviour to guard.

## Risks

| Risk | Answer |
|---|---|
| The Tracey release goes away or changes | The SHA-256 in the workflow stops a changed file. A copy of the release in the repository's own storage can replace the download |
| A requirement id changes, and many markers then name an unknown id | Tracey's version suffix (`+2`) changes the text and keeps the id. A real rename changes the markers in the same commit, and the gate shows each one |
| The markers add noise to the code | One comment line above each item. The 60 markers of the core specs are the measure: if the code owners reject them, this change stops before the SDK and Network Monitor V3 steps |
| A marker with no item below it passes the gate | Tracey 1.4.0 does not check it. Report it to bearcove/tracey |
