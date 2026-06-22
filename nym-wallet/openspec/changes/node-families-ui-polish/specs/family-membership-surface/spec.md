## ADDED Requirements

### Requirement: Operator membership shown in My family tab
The system SHALL display the operator's current family membership status (and Leave action) in the "My family" tab, not in the "Node invites" tab.

#### Scenario: Member node shown in My family tab
- **WHEN** the account controls a node that is a member of a family
- **THEN** the "My family" tab SHALL display a card stating "Node {id} is a member of {family name}"
- **THEN** the "Node invites" tab SHALL NOT display a "Current family" section

#### Scenario: Leave family button is compact
- **WHEN** the membership card is displayed
- **THEN** the "Leave family" button SHALL NOT be full-width
- **THEN** it SHALL appear left-aligned, similar in size to "Save changes" and "Send invite"

#### Scenario: Non-member node shows no membership card
- **WHEN** the account controls a node that is not a member of any family
- **THEN** no membership card SHALL appear in the "My family" tab
