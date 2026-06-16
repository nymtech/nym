## 1. Types & request bindings

- [x] 1.1 Add TS types in `src/types` for `NodeFamily`, `FamilyMembership`, `FamilyInvitation`, `PendingFamilyInvitationDetails`, `PastFamilyInvitation` (with status), `PastFamilyMember`, and contract `Config` (fee + limits)
- [x] 1.2 Add `src/requests/families.ts` Tauri IPC bindings for execute msgs: createFamily, updateFamily, disbandFamily, inviteToFamily, revokeFamilyInvitation, kickFromFamily, acceptFamilyInvitation, rejectFamilyInvitation, leaveFamily
- [x] 1.3 Add query bindings: getFamilyByOwner, getFamilyMembership, family members paged, pending invitations for family/node paged, past invitations for family paged, past members for family paged, config
- [x] 1.4 Export new requests from `src/requests/index.ts`

## 2. Context, hooks & query keys

- [x] 2.1 Create `src/context/families.tsx` (`FamiliesContext` + provider) exposing owner + operator operations, loading/error/refresh; wire into `src/context/index.tsx`
- [x] 2.2 Add `familyQueryKeys` module mirroring `delegationQueryKeys`
- [x] 2.3 Add TanStack Query read hooks: useFamilyByOwner, useFamilyConfig, useFamilyMembers, usePendingInvitationsForFamily, usePastInvitationsForFamily, usePastMembersForFamily, usePendingInvitationsForNode, useFamilyMembership
- [x] 2.4 Add a `useFamilyMemberList` aggregator hook combining the four section queries (Pending, Joined, Rejected, Removed) into one consumable shape for the UI; each section maps 1:1 to its underlying query (no cross-section deduplication, no priority cascade); Revoked past invitations are not surfaced in any section
- [x] 2.5 Add execute hooks/methods with optimistic refresh + error surfacing for all nine execute msgs

## 3. node-families-contract mock & fixtures (derived from `openspec/specs/node-families-contract/spec.md`)

- [x] 3.1 Create `src/context/mocks/families.fixtures.ts` with a `Config` (create_family_fee, name/description byte limits, default_invitation_validity_secs) and typed fixtures for `NodeFamily`, `FamilyMembership`, `FamilyInvitation`, `PendingFamilyInvitationDetails`, `PastFamilyInvitation` (Accepted/Rejected/Revoked), `PastFamilyMember`
- [x] 3.2 Seed fixtures: a sample owned family; members across Joined and Removed (left + kicked); past invitations as Rejected and Revoked; pending invitations including at least one expired and one active
- [x] 3.3 Add a multi-node operator fixture: two controlled nodes with different invite states (active, expired, none)
- [x] 3.4 Create `src/context/mocks/families.tsx` mirroring the context with `mockSleep` latency and mutable in-memory state (follow the `mocks/bonding.tsx` convention; return `TxResultMock` from execute methods)
- [x] 3.5 Implement mock execute methods that mutate fixtures and honor contract invariants: createFamily, updateFamily (`updated_name`/`updated_description` `Option<String>`; None = unchanged), disbandFamily, inviteToFamily, revokeFamilyInvitation, kickFromFamily, acceptFamilyInvitation, rejectFamilyInvitation, leaveFamily, plus an `onNymNodeUnbond` test helper
- [x] 3.6 Implement mock query functions for every contract query: getFamilyById, getFamilyByName (normalised), getFamilyByOwner, getFamilyMembership, and all paginated queries with exclusive `start_after`, default limit 50, max 100, and `start_next_after`
- [x] 3.7 Enforce contract invariants in the mock: one family per owner, one family per node, monotonic non-recycled ids starting at 1, ASCII name normalisation + global uniqueness, byte-length limits, live `expired = now >= expires_at`, per-`(family, node)` archive counters from 0
- [x] 3.8 Model the contract error set as typed mock errors (InvalidFamilyCreationFee, FamilyNameAlreadyTaken/TooLong, EmptyFamilyName, SenderAlreadyOwnsAFamily, NodeAlreadyInFamily, NodeDoesntExist, PendingInvitationAlreadyExists, ZeroInvitationValidity, InvitationExpired/NotFound, FamilyNotEmpty, SenderDoesntControlNode, NodeNotMemberOfFamily) so warning/error UI states are reachable
- [x] 3.9 Have mock execute returns carry the spec's event names/attributes (family_creation, family_disband, family_invitation, family_invitation_revoked/accepted/rejected, family_member_left/kicked, family_node_unbond_cleanup)

## 4. Owner UI components (from Figma)

- [x] 4.1 Pull owner-side designs via Figma MCP and implement: CreateFamily form (name, description, fee display, balance/fee errors), with byte-limit validation + input sanitisation
- [x] 4.2 EditFamily form (name/description, byte limits, inline over-limit error): send only changed fields as `Some(value)` and unchanged ones as `None`; if nothing changed, do not submit
- [x] 4.3 InviteNode form (node ID input, validation) with confirmation and the three warning states (already-in-family, non-existent, duplicate pending)
- [x] 4.4 PendingInvites list with withdraw (active, confirmation) and clear-expired (confirmation) actions and `expired` badges
- [x] 4.5 MemberList grouped by Pending/Joined/Rejected/Removed with per-status empty states and refresh
- [x] 4.6 Kick action with confirmation prompt; DeleteFamily action (empty-only, confirmation, `FamilyNotEmpty` error)

## 5. Operator UI components (from Figma)

- [x] 5.1 Pull operator-side designs via Figma MCP and implement: per-node InviteCard (family name, inviting owner, expiry/TTL) with expired = non-actionable
- [x] 5.2 Multi-node grouping of invites
- [x] 5.3 Accept action (confirmation) and Reject action (confirmation)
- [x] 5.4 LeaveFamily action with confirmation

## 6. Family Tab & pages

- [x] 6.1 Add the Family tab, always visible for every wallet account (no eligibility gating)
- [x] 6.2 Owner management page composing components from section 4
- [x] 6.3 Operator invites page composing components from section 5
- [x] 6.4 Route between create entry point and management surface based on ownership

## 7. Storybook (three levels)

- [x] 7.1 Add a `withFamiliesMock` decorator backed by the mock provider
- [x] 7.2 Component stories with explicit state args (empty, loading, error, expired, over-limit, success) for every component in sections 4 & 5
- [x] 7.3 Page stories for the owner management page and operator invites page
- [x] 7.4 Flow stories with `@storybook/test` play functions: owner flow (create → invite → accept → kick → disband) and operator flow (receive → accept/reject → leave)

## 8. Tests

- [x] 8.1 Hook/integration tests (Jest) for every execute method and the status-derivation selector against the mock engine. NOTE: jest is `node` env, `*.test.ts` only (no jsdom/RTL render) — coverage is at the mock-engine + extracted pure `deriveMemberSections` selector level, which is the mock provider's logic. Files: `familiesMockState.test.ts`, `familyMemberSections.test.ts`, `Families/helpers.test.ts` (47 tests).
- [x] 8.2 Storybook interaction tests: assertive `play` functions on the flow stories (run by `@storybook/test-runner` via `pnpm test:storybook`). Requires `pnpm install` + browsers to execute.
- [x] 8.3 Added `@playwright/test` devDep + `playwright.config.ts` (serves Storybook on :6006 via `webServer`). Requires `pnpm install` + `npx playwright install chromium` to run.
- [x] 8.4 Playwright e2e specs `e2e/families.spec.ts` covering owner + operator flows (+ multi-node states) against the flow stories.
- [x] 8.5 Spec scenarios covered in `familiesMockState.test.ts` (create success/fee errors, over-limit + special-char normalisation, invite warnings, revoke active, expired-invite flag, kick, disband empty vs blocked, accept→Joined, reject→Rejected, leave→Removed + rejoin, multi-node) and `helpers.test.ts` (insufficient-balance pre-check, sanitisation). "kick + cancel" cancellation path is UI-only (no contract call) — exercised by the flow stories, not the engine.

## 9. Wiring & verification

- [x] 9.1 App route `/family` uses the real `FamiliesContextProvider` (via `pages/families/FamilyPageRoute.tsx` → `context/FamiliesContextProvider.tsx`); the mock provider is confined to Storybook/tests. The real provider is the ONLY families module importing `./main` (keeps Storybook free of Tauri-runtime code).
- [x] 9.2 Fee + limits are read from contract `Config` via `useFamilyConfig` (`create_family_fee`, `family_name_length_limit`, `family_description_length_limit`); no hardcoded 100 NYM / char counts. (The `?? 30/120` fallbacks are load-time display defaults only; the submitted fee is always `config.data.create_family_fee`.)
- [x] 9.3 `pnpm test` (85/85 green incl. 47 family) and `build-storybook` (succeeds) run clean; tsc + eslint clean. Playwright (`test:e2e`) and `test:storybook` are NOT run here — blocked on `pnpm install` of the new devDeps + `npx playwright install chromium` (no network/browsers in this env).
- [~] 9.4 **Realised by the `node-families-real-ipc` change.** The Rust Tauri command handlers (the missing layer this task waited on) are now implemented in `src-tauri/src/operations/families/` and registered in `main.rs`; the real `FamiliesContextProvider` is wired to them and `controlledNodeIds` derives from the bonded node. The remaining manual IPC smoke on a wired build (create + invite + accept against sandbox) is tracked there as §3.3 / §4 / §5 (needs the native app + sandbox + the funded test account).
- [x] 9.5 **RESOLVED:** `ExecuteMsg::UpdateFamily { updated_name: Option<String>, updated_description: Option<String> }` is confirmed in the `node-families-contract` source (`src/msg.rs`), matching the wallet's `requests/families.ts`, mock execute, and TS types. Verified in `node-families-real-ipc` §6.2.
