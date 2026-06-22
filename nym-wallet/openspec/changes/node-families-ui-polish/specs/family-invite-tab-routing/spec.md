## ADDED Requirements

### Requirement: Freshly-invited node appears in Pending immediately (NYM-1559)
The system SHALL route a node to the Pending section of the Members card immediately after an invitation is sent, and SHALL NOT show it in Joined, Rejected, or Removed until its status actually changes.

#### Scenario: Invited node appears in Pending after invite is sent
- **WHEN** the owner sends an invitation to a node
- **THEN** that node SHALL appear in the Pending subsection of the Members card
- **THEN** that node SHALL NOT appear in the Joined, Rejected, or Removed subsections

#### Scenario: Node moves from Pending to Joined after acceptance
- **WHEN** the invited node accepts the invitation
- **THEN** the node SHALL move from Pending to Joined in the Members card

#### Scenario: Node moves from Pending to Rejected after rejection
- **WHEN** the invited node rejects the invitation
- **THEN** the node SHALL move from Pending to Rejected in the Members card
