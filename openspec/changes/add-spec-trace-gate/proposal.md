## Why

`add-mixnet-core-specs` links 21 requirements to their code and tests with [Tracey](https://github.com/bearcove/tracey) markers: `// r[impl <id>]` above the code that keeps a requirement, and `// r[verify <id>]` above the test that checks it. Nothing checks those links. A marker can name an id that does not exist, and a requirement can change while the code that names it stays the same.

The same change also holds a second copy of the code links: 50 hand-written `realises` edges in `edges.proposed.json`, each naming its code as `<path>#<symbol>`. The two copies can disagree. The rebase of `max/mixnet-core-specs` onto develop on 2026-11-09 showed the cost. Develop (#7386) renamed `SurbAck::construct` to `construct_legacy`. The 2 hand edges that name `SurbAck::construct` now resolve to nothing, and no check failed. The two markers above the function moved with it.

A developer who changes code under a requirement also gets no prompt to check that the requirement still holds.

## What Changes

- The markers become the only source of the links from code to a requirement. `documentation/docs/lib/snippets/tracey.mts` gives the docs records and the coverage report their `realises` edges and witnesses. The code entries leave `edges.proposed.json`. The threat-model entries stay there for the threat-model owner to review.
- `.config/tracey/config.styx` covers 7 specs: `sphinx-packet-format`, `client-cover-traffic`, `mix-node-delay`, `sdk-mixnet-stream`, `nym-network-monitor`, `network-monitor-config-score` and `network-monitor-epoch-performance`. For each spec it lists every code path that holds a marker for that spec.
- A new CI job runs `tracey query validate --deny warnings` on each pull request that changes a spec in the scope, the Tracey config, or Rust code. The job blocks the merge when a reference is broken. It reports a requirement with no code or no test, and does not fail on it.
- The SDK and Network Monitor V3 specs get Tracey ids (61 requirements), then markers in their code.
- A lefthook pre-commit job lists the requirements whose marked code a commit changes, with the text of each one. It never fails the commit.

## Capabilities

### New Capabilities

- `spec-traceability`: the marker format as the source of code-to-requirement links, the blocking reference check, the specs and code paths that it covers, and the advisory note at commit time.

### Modified Capabilities

None. The Tracey ids that the archived specs gain are lines above the requirement text. The text of each requirement stays the same.

## Impact

- `.config/tracey/config.styx`: 7 specs with their code paths. `gateway/**` joins the paths, so the `delay.gateways-no-delay` marker counts.
- `.github/workflows/ci-spec-trace.yml` (new): installs Tracey 1.4.0 from its release, checks its SHA-256, and runs the check.
- `lefthook.yml`: one advisory pre-commit job.
- `documentation/docs/lib/snippets/`: `tracey.mts` gains a `--changed` mode and becomes an input of `records.mts` and `coverage.mts`.
- `openspec/changes/add-mixnet-core-specs/edges.proposed.json`: loses its code entries.
- `openspec/specs/sdk-mixnet-stream`, `nym-network-monitor`, `network-monitor-config-score`, `network-monitor-epoch-performance`: one id line for each requirement.
- Rust code in `sdk/rust/nym-sdk` and `nym-network-monitor-v3`: marker comments only. No product behaviour changes.

## Non-Goals

- A check that the code still does what its requirement says. A marker shows where the code is, not what it does. The witness tests guard behaviour, and `add-codex-assurance` (A2) asks for a canary before a statement is enforced.
- A coverage threshold. An uncovered requirement is reported, not failed, so that ids can land before their markers.
- The other 35 specs in `openspec/specs/`, the older network monitor in `nym-api` (`network-monitor`), and `sdk-service-provider-example`.

## Dependencies

- `add-mixnet-core-specs` provides the 21 core requirements and their markers. When it is archived, the Tracey config follows its specs to `openspec/specs/` in the same commit (see `design.md`, T4).
