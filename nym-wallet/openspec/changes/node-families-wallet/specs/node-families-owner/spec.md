## ADDED Requirements

### Requirement: Family Tab is visible to eligible users and exposes a create entry point

The wallet SHALL display a **Family** tab to eligible users (an account whose connected address either owns a family or controls a bonded node). The tab SHALL present a create-family entry point when the connected address does not already own a family. When the address already owns a family, the tab SHALL show the family management surface instead of the create entry point.

#### Scenario: Eligible user without a family sees the create entry point
- **WHEN** an eligible user opens the Family tab and their address owns no family
- **THEN** the tab renders a "Create family" entry point

#### Scenario: Owner sees management surface instead of create
- **WHEN** the connected address already owns a family
- **THEN** the Family tab renders the family management surface (member list, invite, edit, delete) and not the create entry point

### Requirement: Family owner can create a family with the creation fee

The wallet SHALL allow an eligible user to create a family by submitting a name and description and attaching the contract's configured creation fee (`Config::create_family_fee`, read from chain — NOT a hardcoded amount). The wallet MUST display the required fee before submission, deduct it on success, and show a success confirmation that surfaces the new family. The wallet MUST surface an insufficient-balance error before submitting when the balance is below fee + estimated gas, and MUST surface contract fee errors (`InvalidFamilyCreationFee`, `InvalidDeposit`) clearly.

#### Scenario: Successful creation
- **WHEN** a user with sufficient balance submits a valid name and description with the correct fee attached
- **THEN** the family is created, the fee is deducted, and a success confirmation referencing the new family is shown

#### Scenario: Insufficient balance is surfaced before submission
- **WHEN** the connected account balance is below the creation fee plus estimated gas
- **THEN** the wallet shows a clear insufficient-balance error and does not submit the transaction

#### Scenario: Contract fee error is surfaced
- **WHEN** creation fails with `InvalidFamilyCreationFee` or `InvalidDeposit`
- **THEN** the wallet shows a clear fee error and the family is not created

### Requirement: Family key is generated on creation

On successful family creation the wallet SHALL generate a family key (multisig or standalone, per the Discovery decision) and present it to the owner. The exact key mechanism depends on the `node-families-contract` adding key/delegation support; until then the wallet SHALL treat the family key as an opaque value backed by mocked behavior.

#### Scenario: Family key presented on creation
- **WHEN** a family is created successfully
- **THEN** the wallet generates and displays the associated family key to the owner

### Requirement: Family owner can add and edit the family name and description

The wallet SHALL let the owner set a name and description on creation and edit either after creation. Inputs MUST be validated against the contract byte-length limits (`Config::family_name_length_limit`, `Config::family_description_length_limit`) measured in bytes, and MUST be sanitised so that scripts, control characters, and injection attempts are neutralised before submission. Over-limit input MUST be surfaced with an inline error and MUST NOT be submitted. Editing depends on a contract `UpdateFamily` handler (a flagged contract dependency); until available the edit path SHALL operate against mocked behavior.

#### Scenario: Valid name and description are accepted
- **WHEN** the owner enters a name and description within the byte limits
- **THEN** the input passes validation and is submitted

#### Scenario: Over-limit input is blocked with an inline error
- **WHEN** the owner enters a name or description whose byte length exceeds its configured limit (e.g. a multi-byte emoji pushing the byte count over)
- **THEN** the wallet shows an inline over-limit error and does not submit

#### Scenario: Special characters and scripts are sanitised
- **WHEN** the owner enters input containing HTML/script tags or injection attempts
- **THEN** the wallet neutralises the input and never renders it as executable markup

#### Scenario: Owner edits name and description after creation
- **WHEN** the owner edits the name and/or description of an existing family with valid input
- **THEN** the updated values are persisted and reflected in the family management surface

### Requirement: Family owner can invite a node by node ID

The wallet SHALL let the owner invite a node by entering its node ID, triggering `InviteToFamily` (with optional `validity_secs` for the TTL/nonce). On success the wallet MUST show a confirmation. The wallet MUST NOT send the invite and MUST warn the owner when: the node is already in a family (`NodeAlreadyInFamily`), the node does not exist or is unbonding (`NodeDoesntExist`), or a pending invite from this family already exists (`PendingInvitationAlreadyExists`). Malformed node IDs MUST be surfaced with a clear validation error.

#### Scenario: Successful invite
- **WHEN** the owner enters a valid node ID for an existing, family-free node
- **THEN** the invite is sent and a confirmation is shown

#### Scenario: Node already in a family is warned, not invited
- **WHEN** the entered node is already a member of a family
- **THEN** the wallet shows an "already in family" warning and does not send the invite

#### Scenario: Non-existent node is warned, not invited
- **WHEN** the entered node does not exist or is unbonding
- **THEN** the wallet shows a "node does not exist" warning and does not send the invite

#### Scenario: Invalid node ID is rejected
- **WHEN** the owner enters a malformed node ID
- **THEN** the wallet shows a clear validation error and does not submit

### Requirement: Family owner can withdraw pending invites and clear expired ones

The wallet SHALL list the family's pending invitations with their expiry state (using the contract `expired` flag). For an active (not-yet-expired) invite the owner SHALL be able to withdraw it via `RevokeFamilyInvitation` behind a confirmation prompt. Expired invites SHALL be shown as expired with a dismiss/clear option, also behind a confirmation prompt. After either action the invite MUST be removed from the pending list and the displayed contract state refreshed.

#### Scenario: Withdraw an active invite
- **WHEN** the owner withdraws a pending, not-yet-expired invite and confirms the prompt
- **THEN** the invite is revoked on-chain, removed from the pending list, and the state refreshes

#### Scenario: Expired invite shows as expired with a clear option
- **WHEN** a pending invite's `expired` flag is true
- **THEN** the wallet displays it as expired and offers a dismiss/clear action

#### Scenario: Clearing an expired invite requires confirmation
- **WHEN** the owner clears an expired invite
- **THEN** a confirmation prompt is shown, and on confirm the invite is removed from the pending list and the state refreshes

### Requirement: Family owner can view the member list grouped by status

The wallet SHALL display all nodes associated with the family grouped into four statuses: **Pending** (active pending invitations), **Joined** (current members), **Rejected** (invitations the node declined), and **Removed** (members that left or were kicked). The list SHALL refresh to reflect current contract state and SHALL render a per-status empty state when a group has no entries. Statuses are derived from the contract queries: pending invitations, current members, and the past-invitation / past-member archives.

#### Scenario: Members are grouped by status
- **WHEN** the owner opens the member list
- **THEN** nodes appear under Pending, Joined, Rejected, and Removed according to their current contract state

#### Scenario: Empty status shows an empty state
- **WHEN** a status group has no entries (e.g. no pending invites)
- **THEN** the wallet renders a per-status empty state for that group

#### Scenario: List reflects state after an action
- **WHEN** the underlying contract state changes (invite accepted, member kicked, etc.) and the list refreshes
- **THEN** the affected node appears under its new status

### Requirement: Family owner can remove a node from the family

The wallet SHALL let the owner remove (kick) a Joined member via `KickFromFamily`, behind a confirmation prompt. On confirmation the kick is submitted and the node MUST move to **Removed** in the member list. Cancelling the prompt MUST make no contract call and leave state unchanged.

#### Scenario: Successful removal
- **WHEN** the owner kicks a member and confirms the prompt
- **THEN** `KickFromFamily` is triggered and the node moves to Removed in the member list

#### Scenario: Cancellation makes no change
- **WHEN** the owner opens the removal confirmation prompt and cancels
- **THEN** no contract call is made and the member remains Joined

### Requirement: Family owner can delete an empty family

The wallet SHALL offer a delete-family option to the owner. Deletion (via `DisbandFamily`) SHALL be permitted only when the family has zero members and SHALL be behind a confirmation prompt. Attempting to delete a non-empty family MUST surface a clear error (`FamilyNotEmpty`) and MUST NOT remove the family.

#### Scenario: Successful deletion of an empty family
- **WHEN** the owner deletes a family with zero members and confirms the prompt
- **THEN** `DisbandFamily` is triggered, the family is removed, and the creation fee refund is reflected

#### Scenario: Deleting a non-empty family is blocked
- **WHEN** the owner attempts to delete a family that still has members
- **THEN** the wallet shows a clear `FamilyNotEmpty` error and the family is not removed
