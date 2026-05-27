## 1. Types & request bindings

- [ ] 1.1 Add TS types in `src/types` for `NodeFamily`, `FamilyMembership`, `FamilyInvitation`, `PendingFamilyInvitationDetails`, `PastFamilyInvitation` (with status), `PastFamilyMember`, and contract `Config` (fee + limits)
- [ ] 1.2 Add `src/requests/families.ts` Tauri IPC bindings for execute msgs: createFamily, updateFamily, disbandFamily, inviteToFamily, revokeFamilyInvitation, kickFromFamily, acceptFamilyInvitation, rejectFamilyInvitation, leaveFamily
- [ ] 1.3 Add query bindings: getFamilyByOwner, getFamilyMembership, family members paged, pending invitations for family/node paged, past invitations for family paged, past members for family paged, config
- [ ] 1.4 Export new requests from `src/requests/index.ts`

## 2. Context, hooks & query keys

- [ ] 2.1 Create `src/context/families.tsx` (`FamiliesContext` + provider) exposing owner + operator operations, loading/error/refresh; wire into `src/context/index.tsx`
- [ ] 2.2 Add `familyQueryKeys` module mirroring `delegationQueryKeys`
- [ ] 2.3 Add TanStack Query read hooks: useFamilyByOwner, useFamilyConfig, useFamilyMembers, usePendingInvitationsForFamily, usePastInvitationsForFamily, usePastMembersForFamily, usePendingInvitationsForNode, useFamilyMembership
- [ ] 2.4 Add a `useFamilyMemberList` aggregator hook combining the four section queries (Pending, Joined, Rejected, Removed) into one consumable shape for the UI; each section maps 1:1 to its underlying query (no cross-section deduplication, no priority cascade); Revoked past invitations are not surfaced in any section
- [ ] 2.5 Add execute hooks/methods with optimistic refresh + error surfacing for all nine execute msgs

## 3. node-families-contract mock & fixtures (derived from `openspec/specs/node-families-contract/spec.md`)

- [ ] 3.1 Create `src/context/mocks/families.fixtures.ts` with a `Config` (create_family_fee, name/description byte limits, default_invitation_validity_secs) and typed fixtures for `NodeFamily`, `FamilyMembership`, `FamilyInvitation`, `PendingFamilyInvitationDetails`, `PastFamilyInvitation` (Accepted/Rejected/Revoked), `PastFamilyMember`
- [ ] 3.2 Seed fixtures: a sample owned family; members across Joined and Removed (left + kicked); past invitations as Rejected and Revoked; pending invitations including at least one expired and one active
- [ ] 3.3 Add a multi-node operator fixture: two controlled nodes with different invite states (active, expired, none)
- [ ] 3.4 Create `src/context/mocks/families.tsx` mirroring the context with `mockSleep` latency and mutable in-memory state (follow the `mocks/bonding.tsx` convention; return `TxResultMock` from execute methods)
- [ ] 3.5 Implement mock execute methods that mutate fixtures and honor contract invariants: createFamily, updateFamily (`updated_name`/`updated_description` `Option<String>`; None = unchanged), disbandFamily, inviteToFamily, revokeFamilyInvitation, kickFromFamily, acceptFamilyInvitation, rejectFamilyInvitation, leaveFamily, plus an `onNymNodeUnbond` test helper
- [ ] 3.6 Implement mock query functions for every contract query: getFamilyById, getFamilyByName (normalised), getFamilyByOwner, getFamilyMembership, and all paginated queries with exclusive `start_after`, default limit 50, max 100, and `start_next_after`
- [ ] 3.7 Enforce contract invariants in the mock: one family per owner, one family per node, monotonic non-recycled ids starting at 1, ASCII name normalisation + global uniqueness, byte-length limits, live `expired = now >= expires_at`, per-`(family, node)` archive counters from 0
- [ ] 3.8 Model the contract error set as typed mock errors (InvalidFamilyCreationFee, FamilyNameAlreadyTaken/TooLong, EmptyFamilyName, SenderAlreadyOwnsAFamily, NodeAlreadyInFamily, NodeDoesntExist, PendingInvitationAlreadyExists, ZeroInvitationValidity, InvitationExpired/NotFound, FamilyNotEmpty, SenderDoesntControlNode, NodeNotMemberOfFamily) so warning/error UI states are reachable
- [ ] 3.9 Have mock execute returns carry the spec's event names/attributes (family_creation, family_disband, family_invitation, family_invitation_revoked/accepted/rejected, family_member_left/kicked, family_node_unbond_cleanup)

## 4. Owner UI components (from Figma)

- [ ] 4.1 Pull owner-side designs via Figma MCP and implement: CreateFamily form (name, description, fee display, balance/fee errors), with byte-limit validation + input sanitisation
- [ ] 4.2 EditFamily form (name/description, byte limits, inline over-limit error): send only changed fields as `Some(value)` and unchanged ones as `None`; if nothing changed, do not submit
- [ ] 4.3 InviteNode form (node ID input, validation) with confirmation and the three warning states (already-in-family, non-existent, duplicate pending)
- [ ] 4.4 PendingInvites list with withdraw (active, confirmation) and clear-expired (confirmation) actions and `expired` badges
- [ ] 4.5 MemberList grouped by Pending/Joined/Rejected/Removed with per-status empty states and refresh
- [ ] 4.6 Kick action with confirmation prompt; DeleteFamily action (empty-only, confirmation, `FamilyNotEmpty` error)

## 5. Operator UI components (from Figma)

- [ ] 5.1 Pull operator-side designs via Figma MCP and implement: per-node InviteCard (family name, inviting owner, expiry/TTL) with expired = non-actionable
- [ ] 5.2 Multi-node grouping of invites
- [ ] 5.3 Accept action (confirmation) and Reject action (confirmation)
- [ ] 5.4 LeaveFamily action with confirmation

## 6. Family Tab & pages

- [ ] 6.1 Add the Family tab, always visible for every wallet account (no eligibility gating)
- [ ] 6.2 Owner management page composing components from section 4
- [ ] 6.3 Operator invites page composing components from section 5
- [ ] 6.4 Route between create entry point and management surface based on ownership

## 7. Storybook (three levels)

- [ ] 7.1 Add a `withFamiliesMock` decorator backed by the mock provider
- [ ] 7.2 Component stories with explicit state args (empty, loading, error, expired, over-limit, success) for every component in sections 4 & 5
- [ ] 7.3 Page stories for the owner management page and operator invites page
- [ ] 7.4 Flow stories with `@storybook/test` play functions: owner flow (create → invite → accept → kick → disband) and operator flow (receive → accept/reject → leave)

## 8. Tests

- [ ] 8.1 Hook/integration tests (Jest + RTL) for every execute method and the status-derivation selector against the mock provider
- [ ] 8.2 Storybook interaction tests assert play-function outcomes for component and page stories
- [ ] 8.3 Add Playwright as a dev dependency and a config that serves the static Storybook build
- [ ] 8.4 Playwright e2e specs covering the owner flow and operator flow against the flow stories
- [ ] 8.5 Test the spec scenarios explicitly: successful create, insufficient balance, over-limit/special-char input, invite warnings, withdrawal of active invite, expired-invite state, kick + cancel, delete empty vs blocked non-empty, accept→Joined, reject→Rejected, leave→Removed + can rejoin, multi-node invite states

## 9. Wiring & verification

- [ ] 9.1 Replace mock provider with the real `FamiliesContext` in the app (mocks remain for Storybook/tests)
- [ ] 9.2 Confirm fee/limits are read from contract config, not hardcoded
- [ ] 9.3 Run `pnpm test`, `build-storybook`, and Playwright; fix failures
- [ ] 9.4 Manual Tauri smoke check of the IPC wiring for at least create + invite + accept (since e2e runs against Storybook, not native)
- [ ] 9.5 On rebase onto the contract change that adds `UpdateFamily`: verify the `ExecuteMsg::UpdateFamily` variant exists, confirm fields are exactly `updated_name: Option<String>` and `updated_description: Option<String>` with None-means-unchanged semantics, confirm sender-must-be-owner auth, and reconcile any drift in `src/requests/families.ts`, the mock execute, and TS types
