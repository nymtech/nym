## Context

Node Families is a new on-chain capability defined in the `node-families-contract` spec (root openspec). The Nym Wallet is a Tauri + React app. State for each domain is exposed through a React Context provider that calls Tauri IPC `requests` and is mirrored by a mock provider under `src/context/mocks/` (see `bonding.tsx` ↔ `mocks/bonding.tsx`, wired in `src/context/index.tsx`). Reads in newer code use TanStack Query (see `delegationQuery.ts`, `delegationQueryKeys.ts`). Storybook (`@storybook/react-webpack5`, addons: a11y, docs, mcp) is configured but currently only has the default example stories — there is no established pattern yet for rendering domain UI against mock providers. Playwright is not yet a dependency.

This design covers the wallet UI, the hooks/mocks layer, Storybook structure, and the test strategy. UI visuals come from Figma via Figma MCP during apply.

## Goals / Non-Goals

**Goals:**
- A `families` context + hooks that expose the full owner and operator contract surface, each with a mock counterpart.
- Mocked smart-contract fixtures sufficient to drive every story and test without a chain.
- Storybook coverage on three levels: component states, composed pages, full user-flow stories with simulated actions.
- Storybook interaction tests, Playwright e2e flows, and hook/integration tests against mocks.
- UI built from Figma designs.

**Non-Goals:**
- Implementing the smart-contract changes themselves (family key/delegation, `UpdateFamily`) — those are contract-side dependencies tracked separately.
- Reward/redemption flows tied to delegation (explicitly V2 per NYM-1217).
- Real on-chain wiring of the family-key and edit paths before the contract supports them (mock-backed until then).

## Decisions

### D1: One `FamiliesContext` covering both personas, reads via TanStack Query
A single `src/context/families.tsx` (exported from `index.tsx`) exposes owner operations (create, edit, disband, invite, revoke, kick) and operator operations (accept, reject, leave), plus loading/error/refresh. Reads (family-by-owner, members, pending/past invitations, per-node invitations, config) are TanStack Query hooks with a `familyQueryKeys` module mirroring the `delegationQueryKeys` pattern. *Alternative considered:* two separate contexts (owner/operator) — rejected because a single connected account can be both, and the member list needs owner + archive reads together; one context avoids cross-provider coordination.

### D2: Tauri requests in `src/requests/families.ts`, types in `src/types`
Add IPC bindings mirroring `requests/bond.ts` (one function per execute msg + per query), typed against new TS types for `NodeFamily`, `FamilyMembership`, `PendingFamilyInvitationDetails`, `PastFamilyInvitation`, `PastFamilyMember`, and `Config`. Execute calls return `TransactionExecuteResult` like the existing bonding calls.

### D3: A faithful `node-families-contract` mock, derived from the root spec
Following the existing `src/context/mocks` convention (module-level fixtures, a provider that mirrors the real context, execute methods that `setIsLoading` → `mockSleep` → mutate in-memory fixtures → return `TxResultMock`, reads as "fake tauri request" functions), the mock models the **entire** `node-families-contract` surface from `openspec/specs/node-families-contract/spec.md` — not just the happy path. A `withFamiliesMock` Storybook decorator wraps stories in this provider; the same provider backs RTL integration tests. *Alternative considered:* MSW/network mocking — rejected because the data crosses Tauri IPC, not HTTP, so provider-level mocking is the faithful seam.

The mock lives in `src/context/mocks/families.tsx` with fixtures in a co-located `src/context/mocks/families.fixtures.ts`, and reproduces:

- **Config**: `create_family_fee` (DecCoin), `family_name_length_limit`, `family_description_length_limit`, `default_invitation_validity_secs`.
- **Data types**: `NodeFamily { id, name, description, normalised_name, members, created_at, paid_fee, owner }`; `FamilyMembership { family_id, joined_at }`; `FamilyInvitation { family_id, node_id, expires_at }`; `PendingFamilyInvitationDetails { invitation, expired }`; `PastFamilyInvitation { invitation, status: Accepted{at} | Rejected{at} | Revoked{at} }`; `PastFamilyMember { family_id, node_id, removed_at }`.
- **Execute msgs** (mutating fixtures, honoring invariants): `CreateFamily`, `DisbandFamily`, `InviteToFamily`, `RevokeFamilyInvitation`, `KickFromFamily`, `AcceptFamilyInvitation`, `RejectFamilyInvitation`, `LeaveFamily`, plus an `OnNymNodeUnbond` test helper to simulate the mixnet cleanup callback. (`UpdateConfig` is admin-only and out of the wallet's scope.)
- **Queries**: `GetFamilyById`, `GetFamilyByName` (normalised lookup), `GetFamilyByOwner`, `GetFamilyMembership`, and the paginated `GetFamiliesPaged`, `GetFamilyMembersPaged`, `GetAllFamilyMembersPaged`, `GetPendingInvitation(s)ForFamilyPaged`, `GetPendingInvitationsForNodePaged`, `GetAllPendingInvitationsPaged`, `GetPastInvitationsForFamily/NodePaged`, `GetAllPastInvitationsPaged`, `GetPastMembersForFamily/NodePaged` — all with exclusive `start_after`, default limit 50, max 100, and `start_next_after`.
- **Invariants enforced in the mock** so stories/tests exercise real edge cases: one family per owner, one family per node, monotonic never-recycled family ids starting at 1, ASCII normalisation + global uniqueness of names, byte-length limits on name/description, `expired = now >= expires_at` computed live, per-`(family, node)` archive counters starting at 0, and the disband/leave/kick/unbond archival transitions.
- **Errors**: a typed error set mirroring the contract (`InvalidFamilyCreationFee`, `FamilyNameAlreadyTaken`, `FamilyNameTooLong`, `EmptyFamilyName`, `SenderAlreadyOwnsAFamily`, `NodeAlreadyInFamily`, `NodeDoesntExist`, `PendingInvitationAlreadyExists`, `ZeroInvitationValidity`, `InvitationExpired`, `InvitationNotFound`, `FamilyNotEmpty`, `SenderDoesntControlNode`, `NodeNotMemberOfFamily`, etc.) so warning/error states are reachable from mocked calls.
- **Events**: mock execute returns carry the spec's event names/attributes (`family_creation`, `family_disband`, `family_invitation`, `family_invitation_revoked/accepted/rejected`, `family_member_left/kicked`, `family_node_unbond_cleanup`) so any UI/indexer assertions can verify them.

### D4: Member-status derivation
The four UI statuses map to contract reads: **Pending** = `GetPendingInvitationsForFamilyPaged` (carry the `expired` flag), **Joined** = `GetFamilyMembersPaged`, **Rejected** = `GetPastInvitationsForFamilyPaged` filtered to `Rejected` status, **Removed** = `GetPastMembersForFamilyPaged` (left/kicked) plus `Revoked` past invitations where relevant. The derivation lives in a selector hook so both UI and tests share one definition.

### D5: Storybook three-level structure
- **Components** (`src/components/families/*.stories.tsx`): each component with explicit state args (empty, loading, error, expired, over-limit, success).
- **Pages** (`src/pages/families/*.stories.tsx`): composed surfaces (owner management page, operator invites page) backed by the mock provider.
- **Flows** (`*.flow.stories.tsx`): play functions (`@storybook/test`) that perform the user actions end to end (create → invite → accept → kick → disband; operator: receive → accept/reject → leave).

### D6: Playwright runs against the static Storybook build
Because the production app is Tauri (not a plain web target), Playwright e2e specs run against `build-storybook` served statically, exercising the flow stories as real browser sessions. This gives deterministic, chain-free e2e without packaging Tauri. *Alternative considered:* `tauri-driver`/WebDriver against the native app — heavier, flaky in CI, and unnecessary since the contract layer is mocked anyway.

### D7: Creation fee and limits are read from chain config
The UI reads `create_family_fee`, `family_name_length_limit`, and `family_description_length_limit` from contract `Config` (mocked in fixtures), never hardcoding 100 NYM or character counts. Validation is byte-length based to match the contract.

## Risks / Trade-offs

- **[Family key / delegation not in contract spec]** → Model the family key as an opaque value in types/mocks and isolate it behind the `createFamily`/`acceptFamilyInvitation` boundary so a later contract decision (multisig vs standalone) changes only the request layer, not the UI.
- **[No `UpdateFamily` edit handler in contract spec]** → Build the edit UI + mock path now; gate real submission behind a feature check so it is dark until the contract adds the handler.
- **[Status derivation from archives is subtle]** (Rejected vs Revoked vs Removed) → Centralize in one selector hook with unit tests covering each archive→status mapping.
- **[Playwright-vs-Tauri divergence]** → Storybook flows test UI logic against mocks, not the real IPC bridge; a thin set of manual/native smoke checks should still cover Tauri wiring before release.
- **[Fresh Storybook conventions]** → Establish the `withFamiliesMock` decorator and naming once, up front, to avoid per-story drift.

## Migration Plan

Additive only — new tab, context, requests, types, stories, tests. No existing wallet behavior changes. Rollout can be gated by the Family-tab eligibility check (and a feature flag for the contract-dependent edit/key paths) so the surface ships dark until the contract dependencies land. Rollback is removal of the tab entry point.

## Open Questions

- Family key: multisig or standalone? (NYM-1210 "per Discovery decision".) Determines the create/accept request shape.
- Will the contract add an `UpdateFamily` handler for NYM-1211 edits, and what is its message shape / auth?
- Exact eligibility rule for showing the Family tab (owns a family OR controls a bonded node — confirm).
- Figma file/frame URLs for each component and page (to be supplied at apply time via Figma MCP).
- Pagination/refresh strategy for large member lists and invitation archives (cursor-based per the contract's `start_after`).
