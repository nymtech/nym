## Why

QA testing of the Node Families wallet feature against Figma wireframes (node 1859-981) and Jira tickets NYM-1558/1559/1560 revealed several bugs and UX regressions: countdown timers are frozen, loading state bleeds across unrelated buttons, structural sections appear in wrong tabs, data is duplicated, raw contract errors surface to users, and the page alignment is off relative to the design. These need to be fixed before the feature ships.

## What Changes

- **Fix countdown timer** — `nowSecs` is currently computed once on mount via `useMemo` with empty deps; replace with a live interval so pending-invite expiry countdowns actually tick
- **Fix per-action loading state** — the global `isExecuting` flag causes both "Save changes" and "Send invite" buttons to enter loading state simultaneously; split into per-action flags
- **Move "Current family" to My family tab** — the card showing current membership + Leave button currently renders in the "Node invites" tab; move it to the "My family" tab where it belongs, and make the Leave button compact (not full-width)
- **Remove duplicate pending invitations** — both `PendingInvitesList` (standalone card) and `MemberList` (Pending subsection) render the same data; consolidate into `MemberList` only
- **Pre-validate node-in-family before creating** — when the owner's controlled node is already a member of another family, block the create form with an inline warning instead of letting the CosmWasm error surface after submission
- **Fix page padding alignment** — `FamilyPage` uses `p: 4` on all sides; align with other wallet page padding so cards reach the correct horizontal extent per Figma
- **NYM-1558: Auto-add owner's node on family creation** — when the creating account controls a bonded nym-node, include it as an initial member at creation time; surface this in the UI ("Your node X will be added automatically"); owner must still be able to leave their own node from the family
- **NYM-1559: Invited node appears in wrong Members section** — investigate and fix the status mapping / query cache issue that causes a freshly-invited node to show in Joined or Rejected instead of Pending
- **NYM-1560: "See all" truncation for Removed/Rejected** — limit Removed and Rejected sections in `MemberList` to 3 entries by default with a "See all (N)" expansion; never auto-hide entries

## Capabilities

### New Capabilities

- `family-countdown-timer`: Live ticking countdown for pending invite expiry, updated on a 1-second interval
- `family-per-action-loading`: Per-action loading state so individual form buttons reflect only their own in-flight operation
- `family-membership-surface`: Operator membership status (current family + Leave action) surfaced in the My family tab, not Node invites
- `family-pending-dedup`: Single authoritative pending-invites display consolidated into the Members card
- `family-create-prevalidation`: Pre-submission validation that blocks creation when the owner's node is already in a family
- `family-page-alignment`: Page layout padding aligned to Figma and wallet-wide conventions
- `family-auto-add-owner-node`: Owner's nym-node automatically joined at family creation time (NYM-1558)
- `family-invite-tab-routing`: Freshly-invited nodes routed to Pending section immediately (NYM-1559)
- `family-member-list-truncation`: Removed/Rejected sections collapsed to 3 with "See all" expansion (NYM-1560)

### Modified Capabilities

_(No existing spec-level capabilities are changing — all affected code is new from the NYM-1199 branch.)_

## Impact

- `nym-wallet/src/context/FamiliesContextProvider.tsx` — nowSecs interval, per-action loading flags
- `nym-wallet/src/pages/families/FamilyPage.tsx` — page padding
- `nym-wallet/src/pages/families/OwnerManagementPage.tsx` — remove PendingInvitesList, per-action loading, membership pre-check, operator membership card, NYM-1558 auto-add
- `nym-wallet/src/pages/families/OperatorInvitesPage.tsx` — remove "Current family" card (moved to My family tab)
- `nym-wallet/src/components/Families/LeaveFamilyButton.tsx` — compact (non-full-width) button
- `nym-wallet/src/components/Families/MemberList.tsx` — truncated Removed/Rejected sections
- `nym-wallet/src/components/Families/PendingInvitesList.tsx` — removed or repurposed
- `nym-wallet/src/context/families.ts` — extended context type if new flags needed
