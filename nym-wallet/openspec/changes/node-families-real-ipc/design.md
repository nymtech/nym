## Context

Three layers already exist; only the middle one is missing:

- **Frontend** — `src/requests/families.ts` defines all 18 IPC bindings (9 execute + 9 query) with command names + arg shapes; the real `FamiliesContextProvider` already calls them (and `refreshAll()`s queries after each execute). The only stub is `controlledNodeIds = []`.
- **On-chain client** — `validator-client` exposes `NodeFamiliesSigningClient` (create/update/disband/invite/revoke/accept/reject/leave/kick) and `NodeFamiliesQueryClient` (by-id/by-owner/membership/members-paged/pending/past/…). The `node-families-contract` is **deployed to sandbox** (one family, one member).
- **Wallet Tauri layer (MISSING)** — `src-tauri/src/operations/` has `mixnet/`, `vesting/`, etc. but **no `families/`**, so the 18 invoked commands have no handler. This is the entire gap.

## Goals / Non-Goals

**Goals:** implement the 18 Tauri commands over the existing validator-client traits; return frontend-typed results; switch the running app to real data (drop the `controlledNodeIds` stub); verify journeys against sandbox (read smoke + guarded writes).

**Non-Goals:** contract changes; mainnet; UI/`data-testid` changes; replacing the mock entry (it stays for offline e2e).

## Decisions

**D1 — Mirror `operations/mixnet/` for the families command module.**
Add `src-tauri/src/operations/families/{mod,query,execute}.rs`. Each `#[tauri::command]` acquires the account's client from the wallet `State` (as mixnet ops do), calls the corresponding `NodeFamilies{Signing,Query}Client` method, and returns the mapped type. Register all 18 in `main.rs` `invoke_handler`. *Alternative:* one mega-file — rejected for clarity; split execute vs query.

**D2 — `family_events`: rely on post-execute query refresh, parse events best-effort.**
The mock fabricates `FamilyTxResult.family_events`; on chain they'd come from parsing the tx's wasm events. The provider already `refreshAll()`s all family queries after every execute, so the UI re-derives state from queries, not from `family_events`. Decision: return the real `TransactionExecuteResult` fields and populate `family_events` best-effort (parse wasm events if cheap; otherwise empty) — the UI does not depend on it for correctness. Revisit if a view reads `family_events` directly. *Alternative:* full event parsing up front — deferred as unnecessary for the journeys.

**D3 — Derive `controlledNodeIds` from the account's bonded nodes.**
Reuse the existing bonding/account node info the wallet already fetches (the operator persona needs "nodes I control"). Replace the `useMemo(() => [], [])` stub in `FamiliesContextProvider` with that derivation. Keep it resilient when the account controls no nodes.
*Implemented:* consume `useBondingContext().bondedNode` (`[nodeId]` nym-node / `[mixId]` legacy mixnode / `[]` gateway-or-none); the `/family` route is now wrapped in `BondingContextProvider` (it is mounted per-route, not globally). **Reality check:** an account bonds **at most one** node on chain, so `controlledNodeIds` is 0–1 long. The mock's 3-node operator persona is therefore **not reproducible** from a single sandbox account — the multi-node operator journey (§5.3) remains a mock/Storybook-only scenario, and the sandbox operator check is limited to that one node's invites.

**D4 — Generate contract types via ts-rs (`nym-wallet-types`) and reconcile with `src/types/families.ts`.**
`src/types/families.ts` was hand-written for the mock. Prefer generating the canonical shapes from the contract Rust types (ts-rs, as other wallet types are) and reconciling field-by-field (cursors, paged envelope, membership, `FamilyTxResult`). Where the hand-written type and generated type diverge, the generated (contract-truth) shape wins; update the mock/types accordingly so mock and real stay parity.

**D5 — `get_family_config`: no smart query exists → read raw contract state.**
*Confirmed from the contract:* `QueryMsg` has **no** `GetConfig` variant (only `UpdateConfig { config }` execute + `config` in instantiate), and the validator-client query trait has no config getter. So `get_family_config` can't be a normal smart query. Options: (a) **raw contract-state read** of the `Config` `Item` via `query_contract_raw` at its storage key (no contract change; preferred); (b) hardcode/derive client-side (brittle); (c) add a `GetConfig` query to the contract (out of scope here). Plan: (a). The UI only uses `FamilyConfig` for cosmetic fee display, so if (a) proves awkward, `get_family_config` can degrade gracefully without blocking the journeys.

**D6 — Sandbox e2e in two stages; writes are guarded and may stay partly manual.**
(1) **Read smoke** (non-mutating): point a build at the real provider + sandbox, assert the Family page renders the known family/member — automatable and safe. (2) **Write flows**: require a *dedicated funded sandbox test account* (mnemonic via CI secret) and tolerate on-chain latency/fees; they mutate real state, so they must target only that account's family/nodes and clean up (disband/leave) at the end. Iterate until the owner/operator journeys pass. If headless account provisioning isn't available, the write tier stays a documented manual run while the read smoke gates CI. *This is the riskiest part and the one most likely to need iteration.*

**D7 — Fees: follow the wallet's existing execute convention.**
Reuse however `operations/mixnet/` supplies `Option<Fee>` (auto/simulated default) so the families execute commands behave consistently with the rest of the wallet.

## Risks / Trade-offs

- **Type drift mock↔chain** → D4 reconciliation against ts-rs-generated types; a contract-shape test guards it.
- **`family_events` shape mismatch** → D2 leans on query refresh; UI doesn't depend on the fabricated events.
- **Shared sandbox mutation / flakiness / fees** → D6 dedicated funded test account, target-only-self, cleanup; read smoke gates CI, writes non-blocking/manual until provisioning exists.
- **`UpdateFamily` contract shape** (parent §9.5) → verify `update_family` args (`updated_name`/`updated_description: Option`) against the deployed contract on rebase.
- **Sandbox availability/endpoint** → the read smoke must fail soft (skip/non-blocking) if sandbox is down, to avoid false CI reds.

## Migration Plan

Additive: the commands don't exist today, so adding them only enables the already-present provider. Roll out: (1) Rust commands + registration (compiles, app boots), (2) type reconciliation, (3) `controlledNodeIds` derivation, (4) read smoke vs sandbox, (5) guarded write flows + iterate. Rollback = the mock entry still runs the UI offline regardless.

## Open Questions

- ~~Is there a dedicated funded sandbox test account we can use headlessly?~~ **RESOLVED:** yes — a dedicated **sandbox** account is provisioned. Address `n13jtj2unhhtryxllnuc8zkng3nl4xnnjvxe0tzv`, funded with ~101,000 NYM; mnemonic stored in vault as secret **`TAURI-WALLET-MNEMONIC`** (vault.nymte.ch item `95d3d842-90ad-4b6f-8b0c-10f5febce1c3`) — inject via CI secret, never commit it. Inspect with `nym-cli -c sandbox.env account balance <addr>`. Write flows (§5) target only this account; clean up (disband/leave) so it stays reusable.
- ~~Does any view read `FamilyTxResult.family_events`?~~ **RESOLVED:** no — only the type def + the mock/provider *produce* it; nothing reads it, so query-refresh is sufficient (confirms D2, no event parsing needed).
- ~~`get_family_config` mapping?~~ **RESOLVED:** the contract has no config query → read raw contract state (design D5); degrades gracefully (fee display is cosmetic).
- ~~`update_family` arg shape?~~ **RESOLVED:** contract is `UpdateFamily { updated_name, updated_description: Option<String> }` — matches the frontend args + parent §9.5.
- **Still a decision (not a blocker):** replace `src/types/families.ts` wholesale with ts-rs output vs reconcile selectively. Default: reconcile selectively, contract-truth shape wins (design D4).
