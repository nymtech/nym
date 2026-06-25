# family-pending-dedup Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Pending invitations displayed exactly once

The system SHALL display pending family invitations in exactly one place on the owner management surface — as Pending rows in the unified members table — and SHALL NOT render a separate standalone "Pending invites" card. Pending rows are sourced from the live invitations query (not duplicated from the member-list query).

#### Scenario: Pending invite appears only in the members table
- **WHEN** the owner has sent a pending invitation to a node
- **THEN** the invitation SHALL appear as a Pending row in the members table
- **THEN** no separate "Pending invites" card SHALL be rendered on the page

#### Scenario: Withdraw action available from the pending row
- **WHEN** an active pending invitation is shown as a row in the members table
- **THEN** the owner SHALL be able to withdraw (revoke) the invitation from that row's Actions cell

#### Scenario: Expired pending invite can be cleared
- **WHEN** a pending invitation has expired
- **THEN** its row SHALL show an "expired" status and offer a "Clear" action in place of "Withdraw"

#### Scenario: No pending invites shows empty state
- **WHEN** there are no pending invitations and no other member records
- **THEN** the table SHALL show its empty state ("No members or invites yet.")
- **THEN** no standalone pending card SHALL appear

