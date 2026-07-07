## Why

QA testing of the Node Families wallet feature against Figma wireframes (node 1859-981) and Jira tickets NYM-1558/1559/1560 revealed several bugs and UX regressions: countdown timers are frozen, loading state bleeds across unrelated buttons, structural sections appear in wrong tabs, data is duplicated, raw contract errors surface to users, and the page alignment is off relative to the design. These need to be fixed before the feature ships.

## What Changes

- **Fix countdown timer** — `nowSecs` was computed once on mount via `useMemo`; extracted into a live `useNowSecs` hook so pending-invite expiry countdowns actually tick
- **Fix per-action loading state** — the global `isExecuting` flag made unrelated buttons enter loading together; replaced with a single `executingAction` discriminant so each button reflects only its own in-flight action
- **Move "Current family" to My family tab** — current membership + Leave moved out of the Invites tab into the My family tab via `MyNodeFamilySection`, in its own bordered panel; the Leave button is compact and names the family
- **Remove duplicate pending invitations** — `MemberList` + standalone `PendingInvitesList` replaced by one delegations-style `FamilyMembersTable`; pending invites appear exactly once, with an inline Withdraw/Clear action
- **Pre-validate node-in-family before creating** — when the owner's controlled node is already in a family, hide the create form entirely and show the existing-membership panel instead of letting the CosmWasm error surface
- **Fix page alignment** — `FamilyPage` now uses the shared `PageLayout` so cards align with the rest of the wallet
- **NYM-1558: Auto-add owner's node on family creation** — implemented atomically in the node-families contract (PR #6891): the owner's bonded, not-unbonding node is enrolled as the founding member; the wallet just creates the family and the node appears as Joined. Owner can still leave their own node
- **NYM-1559: Invited node appears in wrong Members section** — fixed the status-to-section derivation so a freshly-invited node shows in Pending (and a re-joined node isn't duplicated in history)
- **NYM-1560: clean member history** — superseded the planned "See all" truncation with the unified `FamilyMembersTable` plus joined/history de-duplication, so stale history entries don't clutter the list
- **Invite notification badge** — count badge (active invites only) on both the sidebar "Family" entry and the Family page's "Invites" sub-tab, so users know how many invites need addressing

## Capabilities

### New Capabilities

- `family-countdown-timer`: Live ticking countdown for pending invite expiry, via a shared `useNowSecs` hook
- `family-per-action-loading`: Single `executingAction` discriminant so each action button reflects only its own in-flight operation
- `family-membership-surface`: Controlled-node membership (current family + Leave action) surfaced in the My family tab, not Invites
- `family-pending-dedup`: Pending invites shown exactly once, as rows in the unified members table
- `family-create-prevalidation`: Create form hidden (replaced by the membership panel) when the owner's node is already in a family
- `family-page-alignment`: Page layout aligned to the wallet-wide `PageLayout`
- `family-auto-add-owner-node`: Owner's nym-node auto-enrolled at family creation, atomically by the contract (NYM-1558)
- `family-invite-tab-routing`: Freshly-invited nodes routed to Pending immediately (NYM-1559)
- `family-members-table`: Single delegations-style members table (Node / Status / Actions) with joined/history de-duplication (supersedes NYM-1560 "See all")
- `family-nav-invite-badge`: Count badge (active invites only) on the Family nav entry and the Invites sub-tab

### Modified Capabilities

- The NYM-1558 auto-enrolment is a contract-side change and modifies the `node-families-contract` capability (the `CreateFamily` requirement). That delta lives in `openspec/specs/node-families-contract/spec.md` (PR #6891); the wallet's `family-auto-add-owner-node` capability documents how the wallet relies on it.

## Impact

- `nym-wallet/src/context/families.tsx` — `executingAction` discriminant, `nowSecs`, `usePendingInviteCountForNodes`, resilient member-list query
- `nym-wallet/src/context/familyMemberSections.ts` — `deriveMemberSections` drops currently-joined nodes from rejected/removed
- `nym-wallet/src/hooks/useNowSecs.ts` — new ticking-clock hook
- `nym-wallet/src/hooks/useControlledNodeIds.ts` — new provider-free controlled-node-ids hook (for the always-on nav)
- `nym-wallet/src/pages/families/FamilyPage.tsx` — `PageLayout`, "Invites" tab rename, Invites sub-tab count badge
- `nym-wallet/src/pages/families/OwnerManagementPage.tsx` — membership panel, create pre-check (hide form), single members table, per-action loading
- `nym-wallet/src/pages/families/OperatorInvitesPage.tsx` — "Current family" block removed (moved to My family tab)
- `nym-wallet/src/pages/families/FamilySettingsPage.tsx` — Edit + Dissolve moved here
- `nym-wallet/src/components/Families/FamilyMembersTable.tsx` — new unified members table (replaces MemberList + PendingInvitesList)
- `nym-wallet/src/components/Families/MyNodeFamilySection.tsx` — membership panel (own vs another wallet's family)
- `nym-wallet/src/components/Families/LeaveFamilyButton.tsx` — compact, names the family
- `nym-wallet/src/components/Families/InviteNotificationBadge.tsx` — new styled mint count badge
- `nym-wallet/src/components/Families/helpers.ts` — `inviteWarningFromError` maps raw contract errors to clean warnings
- `nym-wallet/src/components/Nav.tsx` — count badge on the Family nav entry
- `nym/contracts/node-families/` — auto-enrol owner's node at creation (NYM-1558, PR #6891); specced in `openspec/specs/node-families-contract/spec.md`
