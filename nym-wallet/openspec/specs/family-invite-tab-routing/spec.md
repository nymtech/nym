# family-invite-tab-routing Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Freshly-invited node appears in Pending immediately (NYM-1559)

The system SHALL route a node to the Pending rows of the members table immediately after an invitation is sent, and SHALL NOT show it in Joined, Rejected, or Removed until its status actually changes. Pending rows are derived from the live invitations query (`usePendingInvitationsForFamily`), while joined/rejected/removed rows come from the member-list query; the two are merged into the single table.

#### Scenario: Invited node appears in Pending after invite is sent
- **WHEN** the owner sends an invitation to a node
- **THEN** that node SHALL appear as a Pending row in the members table
- **THEN** that node SHALL NOT appear in the Joined, Rejected, or Removed rows

#### Scenario: Node moves from Pending to Joined after acceptance
- **WHEN** the invited node accepts the invitation
- **THEN** the node SHALL move from a Pending row to a Joined row

#### Scenario: Node moves from Pending to Rejected after rejection
- **WHEN** the invited node rejects the invitation
- **THEN** the node SHALL move from a Pending row to a Rejected row

#### Scenario: A re-invited node that was previously removed/rejected shows once
- **WHEN** a node that was previously removed or rejected is invited again
- **THEN** it SHALL appear once as a Pending row and SHALL NOT also appear in the Removed/Rejected rows (the joined/historical de-duplication keeps the list clean)

