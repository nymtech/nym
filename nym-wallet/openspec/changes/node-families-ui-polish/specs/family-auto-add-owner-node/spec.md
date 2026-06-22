## ADDED Requirements

### Requirement: Owner's nym-node is auto-joined at family creation (NYM-1558)
The system SHALL automatically add the owner's bonded nym-node as a member of a newly created family when the creating account controls a node, with no separate invite step required.

#### Scenario: Node auto-added after creation
- **WHEN** an account that controls a bonded nym-node creates a family
- **THEN** that node SHALL appear in the Joined section of the Members card immediately after creation
- **THEN** the owner SHALL NOT need to send a separate invite to their own node

#### Scenario: Create form informs user about auto-add
- **WHEN** the account controls a bonded nym-node and the Create family form is displayed
- **THEN** the form SHALL display a helper message "Your node {id} will be added automatically"

#### Scenario: Owner can leave their own node from the family
- **WHEN** the owner's node was auto-joined at creation
- **THEN** the owner SHALL be able to remove/leave their own node using the standard leave mechanism
- **THEN** the family SHALL continue to exist after the owner's node exits

#### Scenario: No auto-add when account has no bonded node
- **WHEN** the creating account does not control any bonded node
- **THEN** no auto-add occurs and the family is created normally

#### Scenario: Partial failure surfaced to user
- **WHEN** family creation succeeds but the subsequent invite or accept call fails
- **THEN** the system SHALL show an error notification describing the partial failure
- **THEN** the owner's node SHALL appear in Pending (recoverable via Node invites tab)
