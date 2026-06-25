# family-create-prevalidation Specification

## Purpose
TBD - created by archiving change node-families-ui-polish. Update Purpose after archive.
## Requirements
### Requirement: Hide the Create form when the owner's node is already in a family

The system SHALL prevent the owner from attempting to create a family when their controlled nym-node is already a member of a family. Rather than letting the create form submit and surface a raw CosmWasm error, the wallet SHALL hide the create-family fields entirely and instead show the node's existing membership panel (with its Leave action), so the screen stays clean and the next step is obvious.

#### Scenario: Membership panel shown instead of the create form
- **WHEN** the account controls a node that is currently a member of a family
- **THEN** the Create family form fields SHALL NOT be rendered
- **THEN** the node's membership panel (`MyNodeFamilySection`, including a Leave action) SHALL be shown in its place

#### Scenario: Create form shown when node is not a member
- **WHEN** the account controls a node that is not a member of any family
- **THEN** the Create family form SHALL be shown and submittable (subject to other validation)

#### Scenario: Create form shown while membership query is loading
- **WHEN** the membership query has not yet resolved
- **THEN** the wallet SHALL NOT block on the membership check (it only hides the form once a blocking membership is confirmed)

#### Scenario: No controlled node — no membership check
- **WHEN** the account does not control any bonded node
- **THEN** no membership check SHALL occur and the create form SHALL behave normally

#### Scenario: Submission guard as defence-in-depth
- **WHEN** a create is somehow triggered while the node is in a family
- **THEN** `handleCreate` SHALL short-circuit and not send the transaction (the contract would reject it anyway)

