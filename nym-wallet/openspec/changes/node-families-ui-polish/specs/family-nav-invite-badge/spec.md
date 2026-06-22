## Capability

`family-nav-invite-badge` — Notification badge that tells the user, at a glance, how many family invitations are waiting on a decision. Surfaced both in the always-on sidebar and on the Family page's "Invites" sub-tab.

## Requirements

- **R1** When the connected account controls a bonded node that has one or more active (non-expired) pending family invitations, the "Family" entry in the sidebar navigation MUST display a mint notification badge.
- **R2** The badge MUST show the count of active pending invitations inside it (not just a bare dot), so the user knows how many invites need addressing.
- **R3** The same badge MUST also appear on the "Invites" sub-tab label within the Family page, reflecting the same count, so the user can find the pending invites once they land on the page.
- **R4** Only active (non-expired) invitations count toward the badge. Expired invitations cannot be accepted, so they MUST NOT contribute to the count.
- **R5** The badge MUST disappear (count of 0 renders nothing) when there are no active pending invitations, e.g. after the user accepts, rejects, withdraws, or the invite expires.
- **R6** If the account controls no bonded node, or the pending-invite query has not yet resolved, no badge is shown (silent, no spinner in the nav).
- **R7** The badge MUST NOT require the user to have visited the `/family` route first — the sidebar badge reflects live data from initial page load.
- **R8** The sidebar implementation MUST NOT depend on `FamiliesContextProvider` (which is route-scoped to `/family`); it derives the controlled node ids independently so it can run app-wide.
- **R9** The sidebar and the "Invites" sub-tab MUST stay in lockstep: both read from the same shared pending-invite query cache so they refresh together on the same invalidation.

## Scenarios

#### Scenario: Sidebar shows the active invite count on load
- **WHEN** the connected account's bonded node has 2 active pending invitations and the wallet loads
- **THEN** the "Family" sidebar entry SHALL display a mint badge reading "2" without the user visiting `/family` first

#### Scenario: Invites sub-tab mirrors the count
- **WHEN** the user opens the Family page with active pending invitations present
- **THEN** the "Invites" sub-tab label SHALL display the same mint count badge

#### Scenario: Expired invites are not counted
- **WHEN** the only pending invitations for the node have expired
- **THEN** no badge SHALL be shown in either the sidebar or the Invites sub-tab

#### Scenario: Badge clears after the last invite is resolved
- **WHEN** the user accepts or rejects the last remaining active invitation
- **THEN** the badge SHALL disappear from both the sidebar and the Invites sub-tab on the next query refresh

#### Scenario: No bonded node, no badge
- **WHEN** the account controls no bonded node
- **THEN** no badge SHALL appear and the nav SHALL render normally

## Out of Scope

- Badge for pending invitations sent by an owner (outbound pending invites) — this covers inbound operator invites only
- Persistent notification history or unread tracking

## Implementation notes

- `usePendingInviteCountForNodes(nodeIds)` (`src/context/families.tsx`) sums non-expired pending invites across the given nodes using `useQueries`, sharing the `pendingForNode` query cache.
- `useControlledNodeIds()` (`src/hooks/useControlledNodeIds.ts`) resolves the controlled node ids for the sidebar without the Bonding/Families providers.
- `InviteNotificationBadge` (`src/components/Families/InviteNotificationBadge.tsx`) is the styled mint MUI `Badge`; it hides itself at count 0.
- Wired into `src/components/Nav.tsx` (sidebar "Family" entry) and `src/pages/families/FamilyPage.tsx` (the "Invites" sub-tab label).
