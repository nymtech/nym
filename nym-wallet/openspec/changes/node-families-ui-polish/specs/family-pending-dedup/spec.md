## ADDED Requirements

### Requirement: Pending invitations displayed exactly once
The system SHALL display pending family invitations in exactly one place on the owner management page — within the Members card — and SHALL NOT render a separate standalone "Pending invites" card.

#### Scenario: Pending invite appears only in Members card
- **WHEN** the owner has sent a pending invitation to a node
- **THEN** the invitation SHALL appear in the "Pending" subsection of the Members card
- **THEN** a separate "Pending invites" card SHALL NOT be rendered on the page

#### Scenario: Withdraw action available from Members card
- **WHEN** a pending invitation is shown in the Members card Pending subsection
- **THEN** the owner SHALL be able to withdraw (revoke) the invitation from that row

#### Scenario: No pending invites shows empty state
- **WHEN** there are no pending invitations
- **THEN** the Pending subsection SHALL show "No pending entries" (or equivalent empty state)
- **THEN** no standalone card SHALL appear
