# node-families-owner Specification

## Purpose
TBD - created by archiving change node-families-wallet. Update Purpose after archive.
## Requirements
### Requirement: Family Tab SHALL always be visible and expose create or management based on ownership

_Design: `2474:1935` (4 user states) → `2474:1945` No family yet, `2474:1980` Owner; `2474:1449` Balance — Family tab; ref `1861:393` (SECTION 1)._

The wallet SHALL display the **Family** tab for every connected wallet account, regardless of whether the account owns a family or controls a bonded node, so that any account can start a new family. When the connected address does not own a family, the tab SHALL present a create-family entry point. When the address already owns a family, the tab SHALL show the family management surface instead of the create entry point.

#### Scenario: Account without a family sees the create entry point
- **WHEN** any connected account opens the Family tab and its address owns no family
- **THEN** the tab is shown and renders a "Create family" entry point

#### Scenario: Owner sees management surface instead of create
- **WHEN** the connected address already owns a family
- **THEN** the Family tab renders the family management surface (member list, invite, edit, delete) and not the create entry point

### Requirement: Family owner SHALL create a family with the creation fee

_Design: ref `1861:638` (SECTION 2 · Create Family · NYM-1210); canonical entry point `2474:1945` (No family yet)._

The wallet SHALL allow an eligible user to create a family by submitting a name and description and attaching the contract's configured creation fee (`Config::create_family_fee`, read from chain — NOT a hardcoded amount). The wallet MUST display the required fee before submission, deduct it on success, and show a success confirmation that surfaces the new family. The wallet MUST surface an insufficient-balance error before submitting when the balance is below fee + estimated gas, and MUST surface contract fee errors (`InvalidFamilyCreationFee`, `InvalidDeposit`) clearly.

The wallet MUST also block creation (inline warning, Create button disabled) when the account's controlled nym-node is already a member of another family — this is a pre-submission guard to avoid a raw contract error. The warning SHALL identify the node and its current family. The guard SHALL be silent while the membership query is loading and SHALL NOT activate when the account has no bonded node.

When the creating account controls a bonded nym-node, that node SHALL be automatically added as a member of the new family at creation time — no separate invite step is required. The create form SHALL display a helper message ("Your node {id} will be added automatically") when this applies. If the auto-add (invite + accept) partially fails after family creation succeeds, the wallet MUST surface an error notification and the node will appear in Pending (recoverable via the Node invites tab).

#### Scenario: Successful creation
- **WHEN** a user with sufficient balance submits a valid name and description with the correct fee attached
- **THEN** the family is created, the fee is deducted, and a success confirmation referencing the new family is shown

#### Scenario: Insufficient balance is surfaced before submission
- **WHEN** the connected account balance is below the creation fee plus estimated gas
- **THEN** the wallet shows a clear insufficient-balance error and does not submit the transaction

#### Scenario: Contract fee error is surfaced
- **WHEN** creation fails with `InvalidFamilyCreationFee` or `InvalidDeposit`
- **THEN** the wallet shows a clear fee error and the family is not created

#### Scenario: Creation blocked when owner's node is already in a family
- **WHEN** the account controls a node that is currently a member of another family
- **THEN** the create form displays an inline warning identifying the node and its current family
- **THEN** the Create button is disabled

#### Scenario: Node auto-added after creation (NYM-1558)
- **WHEN** an account that controls a bonded nym-node successfully creates a family
- **THEN** that node appears in the Joined section of the Members card immediately after creation
- **THEN** the owner does not need to send a separate invite to their own node

#### Scenario: Auto-add partial failure surfaced
- **WHEN** family creation succeeds but the subsequent invite or accept call for the owner's node fails
- **THEN** the wallet shows an error notification describing the partial failure
- **THEN** the node appears in Pending (the owner can complete acceptance from the Node invites tab)

#### Scenario: No auto-add when account has no bonded node
- **WHEN** the creating account does not control any bonded node
- **THEN** no auto-add occurs and the family is created normally without a helper message

### Requirement: Family owner SHALL add and edit the family name and description

_Design: ref `1861:794` (SECTION 3 · NYM-1211 edit); canonical `2474:1980` (Owner state)._

The wallet SHALL let the owner set a name and description on creation and edit either after creation. Inputs MUST be validated against the contract byte-length limits (`Config::family_name_length_limit`, `Config::family_description_length_limit`) measured in bytes, and MUST be sanitised so that scripts, control characters, and injection attempts are neutralised before submission. Over-limit input MUST be surfaced with an inline error and MUST NOT be submitted. Editing uses the contract's `UpdateFamily` handler.

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

### Requirement: Family owner SHALL invite a node by node ID

_Design: ref `1861:1150` (SECTION 4 · Invite Node · NYM-1212), incl. the three warning states; canonical `2474:1980` (Owner state)._

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

### Requirement: Family owner SHALL withdraw pending invites and clear expired ones

_Design: ref `1861:794` (SECTION 3 · roster/invite management) and `1861:1150` (SECTION 4 · pending invite + expired states)._

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

### Requirement: Family owner SHALL view the member list grouped by status

_Design: ref `1861:794` (SECTION 3 · view roster · NYM-1213); canonical `2474:1980` (Owner state)._

The wallet SHALL display the family's records grouped into four sections: **Pending** (active pending invitations), **Joined** (current members), **Rejected** (invitations the node declined), and **Removed** (members that left or were kicked). Each section is sourced from a distinct contract query and paginates independently using the contract's exclusive `start_after` cursor (default page size 50, max 100), fetching subsequent pages via the returned `start_next_after`. Because the contract stores per-`(family, node)` archive records that accumulate over time, a single node MAY appear in more than one section when its history justifies it (e.g., currently Joined and previously Removed); each row represents a record, not a node. `Revoked` past invitations are owner-side actions and SHALL NOT be shown in the member list. The list SHALL refresh to reflect current contract state and SHALL render an empty state for any section with no entries.

#### Scenario: Large section is paginated by cursor
- **WHEN** a section has more entries than one page
- **THEN** the wallet fetches additional pages using `start_after`/`start_next_after` rather than loading the whole section at once

#### Scenario: Records are grouped into sections
- **WHEN** the owner opens the member list
- **THEN** records appear under Pending, Joined, Rejected, and Removed according to which contract query produced them

#### Scenario: Node appears in multiple sections when history justifies it
- **WHEN** a node is currently a member of the family AND has been kicked or has left at some earlier point
- **THEN** it appears as a row in Joined for the current membership AND as a separate row in Removed for the past kick/leave

#### Scenario: Revoked invitations are not shown in the member list
- **WHEN** a node has only past `Revoked` invitations from this family (no current membership, no pending invite, no past membership, no past Rejected invitation)
- **THEN** the node does not appear in the member list

#### Scenario: Empty section shows an empty state
- **WHEN** a section has no entries (e.g. no pending invites)
- **THEN** the wallet renders an empty state for that section

#### Scenario: List reflects state after an action
- **WHEN** the underlying contract state changes (invite accepted, member kicked, etc.) and the list refreshes
- **THEN** the new record appears in its corresponding section, while any pre-existing records for the same node remain in their own sections

### Requirement: Family owner SHALL remove a node from the family

_Design: ref `1861:794` (SECTION 3 · remove member · NYM-1214); canonical `2474:1311` (Member remove state)._

The wallet SHALL let the owner remove (kick) a Joined member via `KickFromFamily`, behind a confirmation prompt. On confirmation the kick is submitted and the node MUST move to **Removed** in the member list. Cancelling the prompt MUST make no contract call and leave state unchanged.

#### Scenario: Successful removal
- **WHEN** the owner kicks a member and confirms the prompt
- **THEN** `KickFromFamily` is triggered and the node moves to Removed in the member list

#### Scenario: Cancellation makes no change
- **WHEN** the owner opens the removal confirmation prompt and cancels
- **THEN** no contract call is made and the member remains Joined

### Requirement: Family owner SHALL delete an empty family

_Design: ref `1861:794` (SECTION 3 · dissolve empty family · NYM-1215); canonical `2474:1305` (Dissolve)._

The wallet SHALL offer a delete-family option to the owner. Deletion (via `DisbandFamily`) SHALL be permitted only when the family has zero members and SHALL be behind a confirmation prompt. Attempting to delete a non-empty family MUST surface a clear error (`FamilyNotEmpty`) and MUST NOT remove the family.

#### Scenario: Successful deletion of an empty family
- **WHEN** the owner deletes a family with zero members and confirms the prompt
- **THEN** `DisbandFamily` is triggered, the family is removed, and the creation fee refund is reflected

#### Scenario: Deleting a non-empty family is blocked
- **WHEN** the owner attempts to delete a family that still has members
- **THEN** the wallet shows a clear `FamilyNotEmpty` error and the family is not removed

