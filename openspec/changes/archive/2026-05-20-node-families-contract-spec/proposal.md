## Why

The node-families CosmWasm contract (`contracts/node-families/`) ships and is live, but the design exists only as Rust source and inline doc comments — there is no externally-readable specification of *what* the contract guarantees, *who* is authorised to do *what*, or *how* it interacts with the mixnet contract. Without that artefact:

- Route-selection logic in clients and the nym-api currently treats node-families data as opaque inputs; reviewers and integrators have no normative reference to check assumptions against.
- The single-commit origin (`a21a01cf1a`, "node families (#6715)") means there is no PR-by-PR trail of design decisions to reverse later — capturing the spec now is materially cheaper than reconstructing it from `git blame` once memory fades.
- Follow-on work (operator verification, route-policy enforcement, indexer expansion in node-status-api) needs a stable interface to plan against.

The goal of this change is to **reverse-engineer specifications** for the on-chain contract that already exists at HEAD on `develop`. No behaviour change is proposed; this is a documentation/spec-only deliverable that ratifies the current implementation as the baseline.

## What Changes

- Introduce a new capability spec `node-families-contract` covering the on-chain CosmWasm contract: instantiation, runtime config, family lifecycle (create/disband), invitation lifecycle (invite/accept/reject/revoke/expire), membership lifecycle (join/leave/kick), the mixnet-contract unbonding callback, and the full read-query surface.
- Document the contract's external invariants (one family per owner, one family per node, globally unique normalised family names, monotonic family ids, sequential per-`(family,node)` archive counters, no background sweeper for expired invitations).
- Document the cross-contract trust boundary with the mixnet contract: which checks the families contract delegates (node existence, node controller, unbonding state) and which it owns (family ownership, invitation state, membership archives).
- Document the event surface emitted by each execute path, since indexers (node-status-api) and downstream tooling consume those names/attributes as a public API.

No code changes. No migrations. No new dependencies.

## Capabilities

### New Capabilities

- `node-families-contract`: the CosmWasm contract that lets node operators declare groupings of co-owned nodes ("families"), issue and resolve invitations, and exposes the queryable state that route-selection logic uses to disallow entry+exit pairs from the same family.

### Modified Capabilities

_None — there are no existing specs in `openspec/specs/` and this change does not alter on-chain behaviour._

## Impact

- **Affected code**: none modified. The spec is derived from `contracts/node-families/` and `common/cosmwasm-smart-contracts/node-families-contract/` at HEAD on `develop` (anchor commit `a21a01cf1a`).
- **Affected consumers** (documented for traceability, not changed): `validator-client` (`NodeFamiliesQueryClient` / `NodeFamiliesSigningClient` traits), `nym-api` (`src/node_families/` cache + HTTP routes), `nym-node-status-api` (`db/queries/node_families.rs` + DVpn gateway routes), and the mixnet contract's unbonding flow which fires `ExecuteMsg::OnNymNodeUnbond`.
- **Dependencies**: none. CosmWasm storage layout (storage-key constants in `common/cosmwasm-smart-contracts/node-families-contract/src/constants.rs`) is part of the spec surface — changing those constants is a breaking change for already-deployed contracts and must be treated as such by any future delta.
- **Non-goals**: route-policy enforcement (client-side or nym-api-side filtering using family data), operator verification, geographic/subnet-distinctness checks, indexer schema, HTTP API shapes. These all consume the contract but live outside its boundary and will get their own specs in follow-on changes.
- **Known limitation — ASCII-only names**: family-name normalisation drops every character that is not an ASCII letter or digit. Non-ASCII letters (`"café"` → `"caf"`, `"Ω-team"` → `"team"`, `"名前"` → `""`) and emoji are stripped entirely; names that normalise to the empty string are rejected with `EmptyFamilyName`. This is intentional, deterministic, and gas-efficient (no Unicode segmentation dependency in the Wasm binary), but it does mean operators picking non-ASCII branding cannot use it as their family name. Documented here so it surfaces in the change log and not just buried inside `normalise_family_name`.
