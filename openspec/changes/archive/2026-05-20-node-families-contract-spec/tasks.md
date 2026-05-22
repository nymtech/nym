## 1. Verify spec coverage against the live contract

- [x] 1.1 Compare each `ExecuteMsg` variant in `common/cosmwasm-smart-contracts/node-families-contract/src/msg.rs` against the spec to confirm every execute path has a corresponding requirement with scenarios.
- [x] 1.2 Compare each `QueryMsg` variant against the spec to confirm every read path is covered (single-family lookups, current-membership listings, pending-invitation listings, past-invitation archive listings, past-member archive listings).
- [x] 1.3 Compare each `NodeFamiliesContractError` variant against the spec to confirm every error has a scenario that triggers it, and that the error name in the scenario matches the enum variant exactly.
- [x] 1.4 Confirm every event name and attribute key in `nym_node_families_contract_common::constants::events` is named verbatim in the spec's events requirement.
- [x] 1.5 Confirm every storage-key constant in `nym_node_families_contract_common::constants::storage_keys` is named verbatim in the spec's storage-keys requirement.

## 2. Cross-check against the contract unit tests

- [x] 2.1 For each `#[test]` in `contracts/node-families/src/storage/mod.rs`, identify the requirement(s) and scenario(s) in the spec that it exercises. Flag any test that asserts behaviour not yet captured in the spec.
- [x] 2.2 Do the same for `contracts/node-families/src/transactions.rs` tests (handler-level).
- [x] 2.3 Do the same for `contracts/node-families/src/queries.rs` tests.
- [x] 2.4 Do the same for `contracts/node-families/src/helpers.rs` tests (notably `normalise_family_name`).
- [x] 2.5 For each spec scenario that has no corresponding contract unit test, decide whether to add a test or annotate the scenario as covered indirectly.

## 3. Validate via openspec tooling

- [x] 3.1 Run `openspec validate node-families-contract-spec` and confirm it reports "valid".
- [x] 3.2 Run `openspec show node-families-contract-spec` and review the rendered output for readability and section ordering.
- [x] 3.3 Run `openspec status --change node-families-contract-spec` and confirm `applyRequires` is satisfied (`tasks` artifact present, all dependencies done).

## 4. Reviewer pass

- [x] 4.1 Walk through `proposal.md` with a reviewer to confirm the "Why" and "Capabilities" sections reflect the team's understanding of what node families is for.
- [x] 4.2 Walk through `design.md` Decisions 1–10 with a reviewer to confirm each rationale matches the team's reasoning at the time the contract was built (anchor commit `a21a01cf1a`).
- [x] 4.3 Walk through `specs/node-families-contract/spec.md` requirement by requirement; for each disagreement, decide whether the spec is wrong (update the spec) or the implementation is wrong (open a follow-on change).
- [x] 4.4 Resolve the three Open Questions in `design.md` or move them to a follow-on change.

## 5. Archive the change

- [x] 5.1 Once reviewed and accepted, run `openspec archive node-families-contract-spec` to promote `specs/node-families-contract/spec.md` into `openspec/specs/node-families-contract/spec.md` as the canonical spec.
- [x] 5.2 Confirm the archived spec is the one referenced by future delta specs (route-policy, operator-verification, etc.).
