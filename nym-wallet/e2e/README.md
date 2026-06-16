# Node Families e2e

Three tiers validate the Family page journeys. They share selectors + fixtures via
[`e2e/shared/families.ts`](./shared/families.ts) so they cannot drift apart.

## Mock-wired build (`WALLET_MOCK_FAMILIES`)

The real wallet bootstrap (`AppProvider`) is Tauri-coupled and login-gated, so it can't run
in a plain browser. A dedicated, flag-gated mock entry solves this (design D2):

- `WALLET_MOCK_FAMILIES=on` adds a `mainMock` webpack entry + `main.mock.html` that mounts
  the real router + `ApplicationLayout` + Family page, but with the Storybook mocks
  (`MockMainContextProvider` + `MockFamiliesContextProvider`) — no Tauri, no login.
- Off (default, and always in production) the mock entry/HTML are never built.
- Persona is chosen at runtime: `main.mock.html?persona=owner|operator|operator-seeded`.

Run it: `pnpm webpack:dev:mock` → http://localhost:9000/main.mock.html?persona=owner#/family

## Tier 1 — Playwright vs dev server (primary, cross-platform)

The main suite. Runs everywhere incl. macOS; this is what gates CI.

```
pnpm --dir .. run build          # once: build workspace packages (@nymproject/types, etc.)
npx playwright install chromium  # once
pnpm test:e2e                    # launches webpack:dev:mock + replays the journeys
```

Config: [`playwright.config.ts`](../playwright.config.ts). Specs: [`families.spec.ts`](./families.spec.ts).

**Visual flow report:** each journey step captures a captioned screenshot (`shot()` in
[`shared/report.ts`](./shared/report.ts)), stitched into a static `e2e-report/index.html`
filmstrip (via global setup/teardown) for smoke inspection. The `build` CI job stages + uploads
`e2e-report/` alongside Storybook (`if: always()`, so a failing run still publishes it).

## Tier 2 — WebdriverIO + tauri-driver (optional, Linux CI only)

Drives the **packaged Tauri binary** in the native WebKitGTK webview. macOS is unsupported
(no WKWebView driver), so `pnpm test:e2e:tauri` **skips** there (design D5). Runs in the
non-blocking `e2e-tauri` CI job under `xvfb`.

- Config: [`../wdio.conf.ts`](../wdio.conf.ts) · Spec: [`../e2e-tauri/families.tauri.ts`](../e2e-tauri/families.tauri.ts)
- Prereqs (Linux): `webkit2gtk-driver`, `cargo install tauri-driver --locked`.
- Binary wiring: `pnpm tauri:build:mock` builds the frontend with `WALLET_MOCK_FAMILIES=on`
  and a Tauri binary whose window boots `main.mock.html` (owner persona) via
  [`../src-tauri/tauri.mock.conf.json`](../src-tauri/tauri.mock.conf.json); the spec navigates to
  other personas in-webview. `wdio.conf.ts` points at `src-tauri/target/release/NymWallet`.
- The run itself is Linux-CI-only (`tauri-driver`; macOS skips via `e2e-tauri/run.mjs`).

  Note: `tauri:build:mock` runs `webpack:prod`, which was previously broken repo-wide — the
  shared `ForkTsCheckerWebpackPlugin` (`write-references` emit mode) + `allowJs` emitted `.js`
  into `src` (polluting it / breaking Jest) and type-checked test files. Fixed in the wallet via
  `outDir: ./.tsbuild` (redirects the emit), `declare module '*.css'` (`src/typings/css.d.ts`),
  and excluding `**/*.test.*` from the type-check program.

## Tier 3 — Sandbox real-IPC read smoke (optional, manual)

The node-families contract is deployed to **sandbox** (currently one family, one member).
This smoke validates the *real* `FamiliesContextProvider` + `requests/families.ts` wiring
that the mock stands in for — separate from, and non-blocking relative to, the mock tiers.

Manual procedure (design D9):

1. Launch the real wallet (`pnpm dev`) and sign into a **sandbox** account (network `SANDBOX`).
2. Open the Family page and confirm it renders the known sandbox family/member via real IPC.
3. **Read-only** — do not create/invite/kick/disband against the shared sandbox.
4. Assert render/shape (or pin the known family id), not exact contents, so a contract
   redeploy doesn't hard-fail.

Promote to a non-blocking CI job only once a sandbox test account can be provisioned
headlessly (mnemonic in CI secrets) — a follow-up, not a blocker.

### Headless equivalent (`sandbox_families_smoke` example)

GUI automation of the real wallet can't run on macOS (tauri-driver is Linux/Windows-only;
Playwright can't drive the native webview), so the read smoke above also exists as a
headless Rust harness that exercises the exact `validator-client` calls the Tauri commands
in `src-tauri/src/operations/families/` wrap. It reads the funded sandbox account mnemonic
from `.env` (`TAURI-WALLET-MNEMONIC`, gitignored — never printed or committed). Run from
the `nym-wallet/` directory:

```sh
# read-only smoke (no state change) — lists the live families/members/config via real IPC
cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke

# owner subset (create → rename → disband, with cleanup) on the funded account
cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke -- --write

# full member/operator journey against FAMILY_OWNER's existing family, using
# ACCOUNT_WITH_BONDED_NODE as the node operator (kick/invite/reject/revoke/accept/leave),
# restoring the node's membership at the end
cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke -- --member

# read-only diagnostics
cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke -- --accounts
cargo run --manifest-path src-tauri/Cargo.toml --example sandbox_families_smoke -- --bond-check <n1-address>
```

`--write` uses `TAURI-WALLET-MNEMONIC` (refuses to start if that account already owns a
family; disbands its throwaway family at the end). `--member` uses `FAMILY_OWNER_MNEMONIC`
(owner) + `ACCOUNT_WITH_BONDED_NODE_MNEMONIC` (operator) to drive all 6 member commands
against the owner's existing family and restore the node's original membership, so a real
family is left unchanged. All mnemonics are read from `.env` (gitignored) and never printed.
Together the two runs cover all 9 execute commands end-to-end on sandbox.

## Real IPC layer (what the mock stands in for)

The 18 commands `requests/families.ts` invokes are implemented in
[`../src-tauri/src/operations/families/`](../src-tauri/src/operations/families/) and registered
in [`../src-tauri/src/main.rs`](../src-tauri/src/main.rs):

- `execute.rs` — 9 state-changing txs over `NodeFamiliesSigningClient` (auto/simulated gas;
  `create_family` attaches the `create_family_fee` as base-denom funds). Returns
  `TransactionExecuteResult` (the optional `family_events` is omitted — the provider
  `refreshAll()`s queries after every execute, so the UI re-derives state from reads).
- `query.rs` — 9 reads over `NodeFamiliesQueryClient`, **normalised at the IPC boundary** to the
  wallet shapes in `src/types/families.ts`: base-denom `Coin` → display `DecCoin`
  (`paid_fee`, `create_family_fee`), per-page contract envelopes → `{ items, start_next_after }`,
  the cw_serde tagged `FamilyInvitationStatus` → `{ kind, at }`. `get_family_config` has no smart
  query, so it reads raw contract state at the `"config"` key.

**Tauri arg casing:** JS passes camelCase keys, Tauri maps them to the commands' snake_case
params — so the `requests/families.ts` execute bindings send `nodeId`/`familyId`/`validitySecs`/
`updatedName`/`updatedDescription`.

**Mock → real switch.** `FamiliesContext.defaultQueries` points at the real `requests/families.ts`;
`MockFamiliesContextProvider` swaps in an in-memory engine. Production `/family`
([`../src/pages/families/FamilyPageRoute.tsx`](../src/pages/families/FamilyPageRoute.tsx)) mounts
`BondingContextProvider` → real `FamiliesContextProvider` → `FamilyPage`; the mock entry
(`main.mock.html`) is offline/e2e-only. `controlledNodeIds` derives from the account's bonded node
(≤1 on chain).

**Contract drift guard.** [`../src/types/families.contract-guard.ts`](../src/types/families.contract-guard.ts)
holds `tsc`-checked assertions between `src/types/families.ts` and the ts-rs-generated contract
types; regenerate (`tools/ts-rs-cli`) on a contract change and the guard flags any divergence.

## Sandbox write flows (guarded, manual)

Mutating journeys (create → invite → accept → kick → disband; accept/leave/reject) need a
**dedicated funded sandbox account** — never a shared/real one:

- Account `n13jtj2unhhtryxllnuc8zkng3nl4xnnjvxe0tzv` (sandbox, ~101k NYM). Mnemonic lives **only**
  in vault secret `TAURI-WALLET-MNEMONIC` — inject via CI secret, never commit it.
- Check state with `nym-cli -c sandbox.env account balance n13jtj2unhhtryxllnuc8zkng3nl4xnnjvxe0tzv`.
- Tolerate on-chain latency (poll/refresh after each execute). Target **only** this account's
  family/nodes and **clean up at the end** (disband / leave) so it stays reusable.
- An account bonds ≤1 node, so the multi-node operator persona can't be reproduced here — that
  journey stays mock/Storybook-only.

Keep this tier manual/non-blocking until headless account provisioning exists.
