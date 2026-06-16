## Why

The Node Families UI is complete and proven against the in-memory mock, but it has never run on real chain data: the frontend invokes 18 Tauri commands (`src/requests/families.ts`) that **don't exist on the Rust side yet**, so the real `FamiliesContextProvider` can't resolve anything. The on-chain pieces are already in place — the `node-families-contract` is deployed to **sandbox**, and `validator-client` already exposes full `NodeFamiliesSigningClient` + `NodeFamiliesQueryClient` traits. The only missing link is the wallet's Tauri command layer that bridges the two. This change adds it and switches the running app from mock to real IPC.

## What Changes

- Add a wallet Tauri command module (`src-tauri/src/operations/families/`) implementing the **18 commands** the frontend already calls (9 execute + 9 query), each delegating to the existing `validator-client` node-families traits via the wallet's signing/query client (mirroring `operations/mixnet/`). Register them in the `invoke_handler` (`main.rs`).
- Map chain results to the frontend's TS contract: execute commands return `FamilyTxResult` (incl. `family_events` parsed from the tx, which the mock currently fabricates); query commands return the existing `NodeFamily` / paged / membership shapes. Confirm `nym-wallet-types` (ts-rs) generation matches `src/types/families.ts`, or reconcile.
- Finish the **real provider**: `FamiliesContextProvider` already wires the 18 requests; remove its `controlledNodeIds` stub (currently `[]`) by deriving controlled node ids from the connected account's bonded nodes, so owner/operator personas work on real data. No change to `FamilyPage`.
- The real provider is **already** the default `/family` route; the mock entry (`main.mock.tsx`) stays for the offline Tier-1 suite. This change makes the *production* app show live family data.
- **Iterate to green against sandbox**: a real-IPC tier — first a read-only smoke (queries render the sandbox family/member), then guarded execute flows against a funded sandbox test account — run until the journeys pass. Replaces the parent change's manual task 9.4 with an automated path where feasible.

## Capabilities

### New Capabilities
- `families-real-ipc`: The wallet's Tauri command layer for the node-families contract (queries + execute via the nyxd/validator client) and the real-data provider wiring that replaces the mock in the running app.

### Modified Capabilities
<!-- None: families-app-mock-build / families-app-e2e (from node-families-tauri-webdriver-e2e) are reused, not changed in requirement. -->

## Impact

- **Rust (`nym-wallet/src-tauri`)**: new `operations/families/{mod.rs,queries.rs,execute.rs}` (or similar); `main.rs` `invoke_handler` additions; uses `nym_validator_client` node-families traits + wallet `State`/signing client; tx-event parsing for `family_events`. Possibly new error variants.
- **Types**: `nym-wallet-types` ts-rs exports for the contract types; reconcile with `src/types/families.ts` (esp. `FamilyTxResult.family_events`, cursors, paged shapes).
- **Frontend (`src`)**: `FamiliesContextProvider` `controlledNodeIds` derivation (drop the `[]` stub); no UI/`data-testid` changes.
- **e2e**: a real-IPC tier (sandbox) — read smoke + guarded write flows; needs a sandbox test account (mnemonic via secrets) and a network/account that tolerates lifecycle mutations.
- **Dependency / sequencing**: builds on `node-families-tauri-webdriver-e2e` (mock app, Tier-1/2 harness) and parent `node-families-wallet` (§9.4/9.5 — real IPC + `UpdateFamily` contract shape). The sandbox contract being live satisfies the external prerequisite.
- **Out of scope**: contract changes (the contract is deployed as-is); mainnet wiring; UI redesign.
