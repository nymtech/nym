# family-nav-invite-badge Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Family nav and Invites sub-tab SHALL show a count badge for active pending invitations

When the connected account controls a bonded node that has one or more active (non-expired) pending family invitations, the wallet MUST display a mint notification badge showing the count on both the sidebar "Family" entry and the Family page "Invites" sub-tab label. Only active invitations count; expired invitations MUST NOT contribute. The badge MUST hide when the count is zero. If the account controls no bonded node, or the pending-invite query has not yet resolved, no badge is shown. The sidebar badge MUST NOT require visiting `/family` first and MUST NOT depend on the route-scoped `FamiliesContextProvider`; both placements MUST read from the same shared pending-invite query cache.

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

