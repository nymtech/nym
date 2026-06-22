## ADDED Requirements

### Requirement: Block family creation when owner's node is already in a family
The system SHALL prevent the owner from submitting the Create family form when their controlled nym-node is already a member of another family, and SHALL display a descriptive inline warning.

#### Scenario: Warning shown when node is already a member
- **WHEN** the account controls a node that is currently a member of a family
- **THEN** the Create family form SHALL display an inline warning alert identifying the node and its current family
- **THEN** the "Create family" button SHALL be disabled

#### Scenario: Warning not shown when node is not a member
- **WHEN** the account controls a node that is not a member of any family
- **THEN** no membership warning SHALL appear on the Create family form
- **THEN** the "Create family" button SHALL be enabled (subject to other validation)

#### Scenario: Warning not shown while membership query is loading
- **WHEN** the membership query has not yet resolved
- **THEN** no warning SHALL be shown (do not block the user on a spinner)

#### Scenario: No controlled node — no membership check
- **WHEN** the account does not control any bonded node
- **THEN** no membership check SHALL occur and the create form SHALL behave normally
