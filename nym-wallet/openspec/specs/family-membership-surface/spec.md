# family-membership-surface Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Operator membership shown in My family tab

The system SHALL surface the controlled node's current family membership (and a Leave action) within the "My family" tab, not the "Invites" tab. It is rendered by `MyNodeFamilySection` inside its own bordered panel (`FamilyContentPanel`), distinct from the owned-family management panel.

#### Scenario: Member node shown in My family tab
- **WHEN** the account controls a node that is a member of a family
- **THEN** the "My family" tab SHALL display a panel describing that node's family membership
- **THEN** the "Invites" tab SHALL NOT display a "Current family" section

#### Scenario: Own family vs another wallet's family are visually distinct
- **WHEN** the controlled node belongs to a family owned by another wallet
- **THEN** its membership panel SHALL be rendered separately from (and clearly distinguishable from) the panel used to manage a family the account owns

#### Scenario: Leave button names the family and is compact
- **WHEN** the membership panel is displayed
- **THEN** the "Leave family" button SHALL include the family name so the user knows exactly which family they are leaving
- **THEN** the button SHALL NOT be full-width — it SHALL be compact, similar in size to "Send invite"

#### Scenario: Non-member node shows no membership panel
- **WHEN** the account controls a node that is not a member of any family
- **THEN** no membership panel SHALL appear in the "My family" tab

