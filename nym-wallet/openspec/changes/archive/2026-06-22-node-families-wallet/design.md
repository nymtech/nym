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
- Implementing the contract-side `UpdateFamily` handler - it lands in a separate contract change that this branch rebases onto before merge.
- Owner-acts-for-node behaviour (V2 per NYM-1217): the future capability for the family owner to perform actions on member nodes, plus any reward/redemption flows tied to it. V1 acceptance is a pure membership record.

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

### D4: Member-list sections map 1:1 to contract queries (one row per record)
The four UI sections each correspond to a distinct contract query, paginating independently via its `start_after` cursor. **Pending**: rows from `GetPendingInvitationsForFamilyPaged`, each carrying the `expired` flag. **Joined**: rows from `GetFamilyMembersPaged`. **Rejected**: rows from `GetPastInvitationsForFamilyPaged` filtered to `Rejected` status. **Removed**: rows from `GetPastMembersForFamilyPaged` (covers both left and kicked). `Revoked` past invitations are owner-side actions and are NOT shown in the member list. Because the contract stores per-`(family, node)` archive records that accumulate (a node may be invited, kicked, re-invited, etc. arbitrarily many times), a single node MAY legitimately appear in more than one section - each row represents a record, not a node. The aggregator hook is therefore a thin pass-through (queries → named sections), not a priority-cascade derivation; UI clarity comes from per-section headings + record timestamps, not from collapsing history.

### D5: Family tab is always visible; UI identifies families by name only
The Family tab renders for **every** wallet account (not gated on owning a family or controlling a node), so any account can start a family: it shows the create entry point when the account owns no family and the management surface when it does. `family_id` is internal only - the UI identifies families by name (globally unique among live families after normalisation) and shows the owner address as supplementary trust context wherever invites are displayed. Names are released for reuse when a family is disbanded, so a past archived record's name may not match the family currently holding that name.

### D6: Large lists are paginated via the contract's exclusive `start_after` cursor
Member lists and invitation archives use the contract's cursor pagination: each page passes `start_after` (exclusive) and reads `start_next_after` from the response to fetch the next page, with the contract's default limit of 50 (max 100). The TanStack Query read hooks expose this as incremental/infinite pagination; the mock honours the same cursor semantics so paging is exercised without a chain.

### D7: Storybook three-level structure
- **Components** (`src/components/families/*.stories.tsx`): each component with explicit state args (empty, loading, error, expired, over-limit, success).
- **Pages** (`src/pages/families/*.stories.tsx`): composed surfaces (owner management page, operator invites page) backed by the mock provider.
- **Flows** (`*.flow.stories.tsx`): play functions (`@storybook/test`) that perform the user actions end to end (create → invite → accept → kick → disband; operator: receive → accept/reject → leave).

### D8: Playwright runs against the static Storybook build
Because the production app is Tauri (not a plain web target), Playwright e2e specs run against `build-storybook` served statically, exercising the flow stories as real browser sessions. This gives deterministic, chain-free e2e without packaging Tauri. *Alternative considered:* `tauri-driver`/WebDriver against the native app — heavier, flaky in CI, and unnecessary since the contract layer is mocked anyway.

### D9: Creation fee and limits are read from chain config
The UI reads `create_family_fee`, `family_name_length_limit`, and `family_description_length_limit` from contract `Config` (mocked in fixtures), never hardcoding 100 NYM or character counts. Validation is byte-length based to match the contract.

## Design Source (Figma)

UI is built from the **Nym 2.0** Figma file, board **"Nym_Wallet – Node families added"**:

- **File key:** `moIK1E6AaXhFz8lI1pZVrI`
- **Board node:** `1859:981`
- **Board URL:** https://www.figma.com/design/moIK1E6AaXhFz8lI1pZVrI/%F0%9F%94%A5%F0%9F%94%A5Nym.2.0%F0%9F%94%A5%F0%9F%94%A5?node-id=1859-981
- Open any frame below via the same URL with `?node-id=<id>` (dash form, e.g. `2474-1935`). Pull frames during apply with the Figma MCP `get_design_context` tool (`fileKey` + `nodeId`).

The board holds two overlapping mockups. The **newer polished wireframe set (`2474:*`, "nym-wallet-ui-wireframes", 28/05) is the canonical build target.** The **ticket-annotated composite (`1861:*`, "family-wallet-composite", 13/05) is the reference** for component-level detail and per-ticket intent (its sections carry the NYM-12xx numbers). Where the two disagree, the `2474:*` set wins; reconcile any drift at apply time.

**Canonical surfaces (`2474:*`):**
- `2474:1935` — Family · all 4 user states → `2474:1945` No family yet · `2474:1980` Owner · `2474:2063` Member, pending invite · `2474:2134` Member, active
- `2474:1360` — Balance — Overview · `2474:1449` — Balance — Family tab
- `2474:1305` — Dissolve · `2474:1311` — Member (remove/offline states)

**Reference composite sections (`1861:*`), ticket-mapped:**
- `1861:393` SECTION 1 — intro / 4 family states
- `1861:638` SECTION 2 — Create Family · **NYM-1210**
- `1861:794` SECTION 3 — Family Detail (roster + settings) · **NYM-1211** edit · **NYM-1213** view roster · **NYM-1214** remove member · **NYM-1215** dissolve empty family
- `1861:1150` SECTION 4 — Invite Node · **NYM-1212**
- `1861:1349` SECTION 5 — Incoming Invite popups · **NYM-1216 / 1217 / 1218**
- `1861:1711` SECTION 6 — Leave family · **NYM-1219**

Also on the board: a wireframe **Components** column (`2474:863`) and ten full-screen render frames (`2386:2352` … `2464:3976`) showing each state in the full app shell. Per-requirement frame links are recorded inline in `specs/node-families-owner/spec.md` and `specs/node-families-operator/spec.md`.

## Risks / Trade-offs

- **[`UpdateFamily` lands in a separate contract change]** → Build the edit UI + mock against the decided shape (see Resolved); verify on rebase per task 9.5 and reconcile any drift in the request binding, mock execute, and TS types. No feature flag needed since the wallet branch only merges after the contract change lands.
- **[Same node may appear in multiple sections]** (e.g., currently Joined and previously Removed) → record timestamps and clear section headings must make the overlap read as history rather than as a duplicate row; aggregator hook is a pass-through, so the risk is purely UX, not data correctness.
- **[Playwright-vs-Tauri divergence]** → Storybook flows test UI logic against mocks, not the real IPC bridge; a thin set of manual/native smoke checks should still cover Tauri wiring before release.
- **[Fresh Storybook conventions]** → Establish the `withFamiliesMock` decorator and naming once, up front, to avoid per-story drift.

## Migration Plan

Additive only — new tab, context, requests, types, stories, tests. No existing wallet behavior changes. This branch rebases onto the `UpdateFamily` contract change before merging, so the edit path is real (not feature-flagged) at ship time. Rollback is removal of the tab entry point.

## Open Questions

_(none open)_

_Resolved:_ **Figma file/frame URLs** for each component and page are now captured — see "Design Source (Figma)" above (file `moIK1E6AaXhFz8lI1pZVrI`, board `1859:981`); the `2474:*` polished set is canonical and the `1861:*` ticket-annotated composite is the per-component reference, with per-requirement node IDs recorded inline in the two spec files. No family key concept in V1 (acceptance is a pure membership record; owner-acts-for-node is V2 per NYM-1217); `family_id` is **internal**, the UI identifies families by **name**; names are unique among **live** families only (released for reuse on disband); the Family tab is **always visible**; large lists are **paginated via the contract's `start_after` cursor** (default 50, max 100); the **`UpdateFamily` message shape** is `ExecuteMsg::UpdateFamily { updated_name: Option<String>, updated_description: Option<String> }` with `None` meaning "field unchanged" and `Some(_)` meaning "set to this value", sender must be the family owner; this lands in a separate contract change and is verified on rebase per task 9.5.
