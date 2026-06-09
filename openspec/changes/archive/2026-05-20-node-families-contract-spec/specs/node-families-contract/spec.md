## ADDED Requirements

### Requirement: Contract instantiation persists the runtime config, mixnet contract address, and admin

The contract SHALL be instantiable exactly once via the standard CosmWasm `instantiate` entry point. The instantiation message SHALL carry a `Config` (creation fee, family-name length limit, family-description length limit, default invitation validity in seconds) and a string-form `mixnet_contract_address`. The handler MUST:

- bech32-validate `mixnet_contract_address` and reject malformed inputs;
- persist the validated mixnet contract address, the `Config`, and `info.sender` as the contract admin;
- record the contract name and `CARGO_PKG_VERSION` via `cw2::set_contract_version`;
- record build information via `set_build_information!`.

#### Scenario: Valid instantiation persists config, mixnet address, and admin
- **WHEN** `instantiate` is called with a valid bech32 mixnet contract address and a well-formed `Config`
- **THEN** the contract stores the `Config` verbatim, the validated `Addr` for the mixnet contract, and sets `info.sender` as the contract admin queryable via `cw-controllers::Admin::assert_admin`
- **AND** `cw2::get_contract_version` returns the crate's `CARGO_PKG_VERSION`

#### Scenario: Invalid mixnet contract address is rejected
- **WHEN** `instantiate` is called with a `mixnet_contract_address` string that fails `Addr::validate`
- **THEN** the call returns an error and no state is persisted

### Requirement: Migration entry point forbids version downgrades

The contract SHALL expose a `migrate` entry point that refreshes recorded build information and uses `cw2::ensure_from_older_version` to guarantee the on-chain contract version is at most the current `CARGO_PKG_VERSION`. Downgrade attempts MUST be rejected.

#### Scenario: Equal or older on-chain version is accepted
- **WHEN** `migrate` is called against an on-chain `cw2` version less than or equal to `CARGO_PKG_VERSION`
- **THEN** the call succeeds and `set_build_information!` updates the stored build metadata

#### Scenario: Newer on-chain version is rejected
- **WHEN** `migrate` is called against an on-chain `cw2` version strictly greater than `CARGO_PKG_VERSION`
- **THEN** the call returns an error and storage is unchanged

### Requirement: Only the contract admin can replace the runtime config

The `ExecuteMsg::UpdateConfig` handler SHALL overwrite the stored `Config` with the value supplied by the caller. The handler MUST call `Admin::assert_admin` and return the underlying `AdminError` (wrapped in `NodeFamiliesContractError::Admin`) when the sender is not the admin.

#### Scenario: Admin replaces the config
- **WHEN** the contract admin sends `ExecuteMsg::UpdateConfig { config }`
- **THEN** the stored `Config` equals the supplied value on subsequent reads

#### Scenario: Non-admin is rejected
- **WHEN** a non-admin sender sends `ExecuteMsg::UpdateConfig`
- **THEN** the call returns an `Admin` error and the stored `Config` is unchanged

### Requirement: Family names are normalised and globally unique under their normalised form

Family names SHALL be normalised to a canonical form by lowercasing ASCII letters, preserving ASCII digits, and dropping every other character (whitespace, punctuation, non-ASCII letters). The normalised form SHALL be stored alongside the user-supplied `name` and SHALL be the key of the `families.normalised_name` unique index. Inputs whose normalised form is the empty string SHALL be rejected with `EmptyFamilyName`. Inputs whose original length exceeds `Config::family_name_length_limit` (measured in **bytes**, via `String::len`) SHALL be rejected with `FamilyNameTooLong { length, limit }`. Creation of a family whose normalised name collides with an existing family SHALL fail with `FamilyNameAlreadyTaken { name, family_id }`.

**Important**: this normalisation is ASCII-only by design. Non-ASCII letters and emoji are stripped entirely (`"café"` → `"caf"`, `"名前"` → `""`, `"⭐stars"` → `"stars"`). Names consisting solely of non-ASCII characters or symbols are rejected as `EmptyFamilyName`. Operators who want a non-ASCII brand cannot encode it in the on-chain family name; the user-supplied `name` field preserves the original formatting but only the normalised ASCII form is enforced for uniqueness and emptiness.

#### Scenario: Normalisation strips punctuation, whitespace, casing, and non-ASCII letters
- **WHEN** the contract normalises any of `"Foo Bar"`, `"foobar"`, `"FOO-BAR"`, or `"  f.o.o.b.a.r  "`
- **THEN** every result equals `"foobar"`

#### Scenario: All-symbol name is rejected
- **WHEN** `CreateFamily` is sent with `name = "!!!---"` (normalises to the empty string)
- **THEN** the call fails with `EmptyFamilyName` and no family is persisted

#### Scenario: Name length is checked against the original string, not the normalised form
- **WHEN** `CreateFamily` is sent with a `name` whose `name.len()` (byte length) exceeds `Config::family_name_length_limit`
- **THEN** the call fails with `FamilyNameTooLong { length, limit }`

#### Scenario: Multi-byte characters count their full byte length toward the limit
- **WHEN** `Config::family_name_length_limit = 8` and `CreateFamily` is sent with `name = "🚀rocket"` (a 4-byte emoji followed by `"rocket"`, totalling `name.len() == 10` bytes — but only 7 user-perceived characters)
- **THEN** the call fails with `FamilyNameTooLong { length: 10, limit: 8 }` even though the visible character count is within the limit
- **AND** had the limit been `>= 10`, the name would have been accepted and the normalised form would be `"rocket"` (the emoji dropped during normalisation)

#### Scenario: Two formattings of the same canonical name collide
- **WHEN** family `A` is created with `name = "Shared"` and `B` later tries to create with `name = "$$shared$$"`
- **THEN** the second call fails with `FamilyNameAlreadyTaken { name: "shared", family_id: A.id }`

### Requirement: An address may own at most one family at a time

A given owner address SHALL own at most one family at any time, enforced by the `families.owner` unique index. `CreateFamily` SHALL pre-check this and fail with `SenderAlreadyOwnsAFamily { address, family_id }` for better error context. The pre-check is in addition to (not instead of) the unique-index defence-in-depth check. Owner-gated handlers (`DisbandFamily`, `InviteToFamily`, `RevokeFamilyInvitation`, `KickFromFamily`) SHALL look up the family by owner and fail with `SenderDoesntOwnAFamily { address }` when none exists.

#### Scenario: Same address cannot create a second family while still owning one
- **WHEN** address `alice` already owns family `A` and `CreateFamily` is sent again with `alice` as sender
- **THEN** the call fails with `SenderAlreadyOwnsAFamily { address: alice, family_id: A.id }`

#### Scenario: Address can create a family again after disbanding its previous one
- **WHEN** `alice` creates family `A` then disbands it, then sends `CreateFamily` again
- **THEN** a new family with a strictly greater id than `A` is created (ids are monotonic and never recycled)

#### Scenario: Owner-gated handler rejects a sender that owns no family
- **WHEN** any of `DisbandFamily`, `InviteToFamily`, `RevokeFamilyInvitation`, or `KickFromFamily` is sent by an address that owns no family
- **THEN** the call fails with `SenderDoesntOwnAFamily { address }`

### Requirement: Family creation requires the configured fee and is rejected if the owner's bonded node is already in a family

`ExecuteMsg::CreateFamily` SHALL require the sender to attach exactly one coin matching `Config::create_family_fee` in both denom and amount. Payment validation MUST go through `cw_utils::must_pay`; mismatches in amount MUST surface as `InvalidFamilyCreationFee { expected, received }`. The handler MUST additionally check that any bonded node the sender controls (as reported by the mixnet contract via `query_nymnode_ownership`) is not currently a member of any family, failing with `AlreadyInFamily { address, node_id, family_id }` otherwise. The handler MUST validate the description length against `Config::family_description_length_limit` and reject overlong descriptions with `FamilyDescriptionTooLong { length, limit }`. On success the handler SHALL emit a `family_creation` event with attributes `family_name`, `owner_address`, `family_id`, `paid_fee`.

#### Scenario: Successful family creation persists the family and emits the event
- **WHEN** a sender with no bonded family-member node and no existing-owned family sends `CreateFamily { name, description }` with the correct fee attached
- **THEN** a new family is persisted with monotonically increasing `id`, the supplied `name` and `description`, the computed `normalised_name`, `members = 0`, `created_at = env.block.time.seconds()`, and `paid_fee` equal to the configured fee
- **AND** the response carries an event named `family_creation` with attributes `family_name`, `owner_address`, `family_id`, `paid_fee`

#### Scenario: Wrong fee denom or missing funds is rejected
- **WHEN** `CreateFamily` is sent with no funds, with multiple denoms, or with a denom different from `Config::create_family_fee.denom`
- **THEN** the call fails with `InvalidDeposit(PaymentError)`

#### Scenario: Wrong fee amount is rejected
- **WHEN** `CreateFamily` is sent with the correct denom but an amount different from `Config::create_family_fee.amount`
- **THEN** the call fails with `InvalidFamilyCreationFee { expected, received }` and the funds remain unspent

#### Scenario: Sender whose bonded node is already in a family is rejected
- **WHEN** `CreateFamily` is sent by an address whose bonded node (per the mixnet contract) is already a member of some family `F`
- **THEN** the call fails with `AlreadyInFamily { address, node_id, family_id: F.id }`

#### Scenario: Overlong description is rejected
- **WHEN** `CreateFamily` is sent with a `description` whose byte length exceeds `Config::family_description_length_limit`
- **THEN** the call fails with `FamilyDescriptionTooLong { length, limit }`

### Requirement: Family ids are monotonic, never recycled, and start at 1

The contract SHALL assign family ids sequentially starting at `1`. The id counter SHALL be persisted as an `Item<NodeFamilyId>` and incremented on each successful family creation. Disbanding a family MUST NOT free or recycle its id. `0` SHALL be reserved as a "no family" sentinel and never assigned.

#### Scenario: First-ever family receives id 1
- **WHEN** the contract is freshly instantiated and the first successful `CreateFamily` runs
- **THEN** the persisted `NodeFamily.id` equals `1`

#### Scenario: Ids are not reused after disband
- **WHEN** family with id `N` is disbanded and a new family is then created
- **THEN** the new family's id is strictly greater than `N` (it is `N+1` if no other families were created in between)

### Requirement: A family can only be disbanded by its owner, must be empty, and refunds the creation fee

`ExecuteMsg::DisbandFamily` SHALL look up the sender's family via the `owner` unique index and fail with `SenderDoesntOwnAFamily` when none exists. The handler MUST reject disbanding while `NodeFamily.members > 0`, failing with `FamilyNotEmpty { family_id, members }`. On success the handler SHALL sweep every still-pending invitation issued by the family (archiving each as `FamilyInvitationStatus::Revoked { at: now }` with status timestamp = `env.block.time.seconds()`), remove the family record, and attach a `BankMsg::Send { to_address: owner, amount: vec![family.paid_fee] }` to the response as a CosmWasm sub-message. The response SHALL include a `family_disband` event with `family_id`, `owner_address`, `refunded_fee` attributes.

**Operational note (non-normative)**: the pending-invitation sweep iterates every entry of `pending_family_invitations` keyed by `family_id` and archives each in a single transaction. Gas cost therefore scales linearly with the number of leftover pending invitations. An owner whose family has accumulated an unusually large number of pending invitations is expected to revoke them in batches (via `RevokeFamilyInvitation`) *before* invoking `DisbandFamily`, since a single disband call that exceeds the per-tx gas limit will fail and leave the family in place. There is no contract-side chunking; the responsibility lies with the caller.

#### Scenario: Owner disbands an empty family and is refunded
- **WHEN** family owner sends `DisbandFamily` while `members == 0`
- **THEN** the family is removed from storage, the response carries a `BankMsg::Send` of `family.paid_fee` to the owner, and a `family_disband` event is emitted

#### Scenario: Refund is attached as a BankMsg::Send sub-message, not a direct bank-module call
- **WHEN** a successful `DisbandFamily` returns its `Response`
- **THEN** the response's `messages` field contains exactly one `CosmosMsg::Bank(BankMsg::Send { to_address: <owner bech32>, amount: vec![<family.paid_fee>] })` and the contract performs no other balance-changing side effect for the refund
- **AND** tx simulators that inspect outbound sub-messages observe the refund there (this is the contract's only avenue for returning funds; integrators rely on it)

#### Scenario: Non-empty family cannot be disbanded
- **WHEN** family owner sends `DisbandFamily` while `members > 0`
- **THEN** the call fails with `FamilyNotEmpty { family_id, members }` and storage is unchanged

#### Scenario: Disbanding sweeps still-pending invitations as Revoked
- **WHEN** family `F` has pending invitations to nodes `n1`, `n2` and the owner sends `DisbandFamily`
- **THEN** the pending entries for `(F.id, n1)` and `(F.id, n2)` are removed from `pending_family_invitations` and archived under `past_family_invitations` with `status = Revoked { at: env.block.time.seconds() }`

### Requirement: A node belongs to at most one family at any time

The `family_members` map SHALL be keyed by `NodeId` alone; the value SHALL carry the `family_id` so the storage layer enforces the one-family-per-node invariant by construction. Handlers that add a membership (`AcceptFamilyInvitation`) and handlers that issue an invitation (`InviteToFamily`) SHALL pre-check the absence of any existing membership for the node and fail with `NodeAlreadyInFamily { node_id, family_id }` otherwise.

#### Scenario: Inviting a node already in a family is rejected
- **WHEN** `InviteToFamily { node_id }` targets a node that already has a membership record for family `F`
- **THEN** the call fails with `NodeAlreadyInFamily { node_id, family_id: F.id }`

#### Scenario: Accepting a second invitation after joining a family is rejected
- **WHEN** node `n` is a member of family `F` and `AcceptFamilyInvitation { family_id: G, node_id: n }` is sent (for a different family `G`)
- **THEN** the call fails with `NodeAlreadyInFamily { node_id: n, family_id: F.id }`

### Requirement: Invitations require an existing family, a bonded target node, and strictly positive validity

`ExecuteMsg::InviteToFamily` SHALL be owner-gated (the family acted on is the sender's owned family, never an argument). The handler MUST:

- compute `validity = validity_secs.unwrap_or(Config::default_invitation_validity_secs)`;
- reject `validity == 0` with `ZeroInvitationValidity`;
- verify `node_id` refers to a currently-bonded, not-unbonding node in the mixnet contract via `MixnetContractQuerier::check_node_existence` (which returns `false` both when no bond exists and when the bond is in the unbonding state), failing with `NodeDoesntExist { node_id }` otherwise;
- verify the node is not already in any family (see "A node belongs to at most one family");
- persist a `FamilyInvitation` with `expires_at = env.block.time.seconds() + validity`;
- reject `(family_id, node_id)` pairs that already have a pending invitation with `PendingInvitationAlreadyExists { family_id, node_id }`;
- emit a `family_invitation` event with `family_id`, `node_id`, `expires_at`.

#### Scenario: Successful invitation persists with the computed expiry
- **WHEN** family owner sends `InviteToFamily { node_id, validity_secs: Some(v) }` and all preconditions hold
- **THEN** a pending invitation for `(owned.id, node_id)` is persisted with `expires_at = env.block.time.seconds() + v`
- **AND** the response carries a `family_invitation` event with `family_id = owned.id`, `node_id`, `expires_at`

#### Scenario: Missing validity falls back to the configured default
- **WHEN** `InviteToFamily { node_id, validity_secs: None }` is sent
- **THEN** the persisted invitation has `expires_at = env.block.time.seconds() + Config::default_invitation_validity_secs`

#### Scenario: Zero validity is rejected
- **WHEN** `InviteToFamily { node_id, validity_secs: Some(0) }` is sent
- **THEN** the call fails with `ZeroInvitationValidity` and no invitation is persisted

#### Scenario: Inviting a non-bonded node is rejected
- **WHEN** `InviteToFamily { node_id }` targets a `node_id` for which the mixnet contract's `check_node_existence` returns `false`
- **THEN** the call fails with `NodeDoesntExist { node_id }`

#### Scenario: Duplicate pending invitation is rejected
- **WHEN** family `F` already has a pending invitation for node `n` and `InviteToFamily { node_id: n }` is sent again by `F`'s owner
- **THEN** the call fails with `PendingInvitationAlreadyExists { family_id: F.id, node_id: n }` (the existing invitation is preserved)

### Requirement: Acceptance and rejection of an invitation are gated on node control

`ExecuteMsg::AcceptFamilyInvitation` and `ExecuteMsg::RejectFamilyInvitation` SHALL each verify that the sender is the controller of the bonded node `node_id` per the mixnet contract (`query_nymnode_ownership` returns a `nym_node` with `node_id == node_id` and `is_unbonding == false`). Failures MUST surface as `SenderDoesntControlNode { address, node_id }`. This single error covers the cases of: sender owns no bonded node, sender owns a different node id, and sender owns the node but it has entered unbonding.

#### Scenario: Non-controller cannot accept an invitation
- **WHEN** `AcceptFamilyInvitation { family_id, node_id }` is sent by an address that does not control `node_id` (per mixnet contract)
- **THEN** the call fails with `SenderDoesntControlNode { address, node_id }`

#### Scenario: Unbonding node cannot accept an invitation
- **WHEN** the sender controls `node_id` but the mixnet contract reports the node as unbonding
- **THEN** `AcceptFamilyInvitation` fails with `SenderDoesntControlNode { address, node_id }`

#### Scenario: Non-controller cannot reject an invitation
- **WHEN** `RejectFamilyInvitation { family_id, node_id }` is sent by an address that does not control `node_id`
- **THEN** the call fails with `SenderDoesntControlNode { address, node_id }`

### Requirement: Accepting an invitation moves it from pending to archived and increments the family member count

A successful `AcceptFamilyInvitation` SHALL:

- load the pending invitation for `(family_id, node_id)`, failing with `InvitationNotFound { family_id, node_id }` if absent;
- check `now < invitation.expires_at`, failing with `InvitationExpired { family_id, node_id, expires_at, now }` otherwise (`now == expires_at` is considered expired);
- remove the entry from `pending_family_invitations`;
- write `family_members[node_id] = FamilyMembership { family_id, joined_at: now }`;
- increment `NodeFamily::members` by 1 (failing with `FamilyNotFound { family_id }` if the family has somehow been removed);
- archive a `PastFamilyInvitation { invitation, status: Accepted { at: now } }` under `((family_id, node_id), counter)` where `counter` is the next free per-`(family, node)` archive slot;
- emit a `family_invitation_accepted` event with `family_id`, `node_id`.

#### Scenario: Happy-path acceptance
- **WHEN** node controller accepts a not-yet-expired pending invitation
- **THEN** the membership is recorded with `joined_at = env.block.time.seconds()`, the family's `members` count is incremented, the pending entry is removed, and the archive contains the invitation with status `Accepted { at: now }`

#### Scenario: Acceptance of an already-expired invitation is rejected
- **WHEN** `AcceptFamilyInvitation` is called with `env.block.time.seconds() >= invitation.expires_at`
- **THEN** the call fails with `InvitationExpired { family_id, node_id, expires_at, now }` and storage is unchanged

#### Scenario: Acceptance with no pending invitation is rejected
- **WHEN** `AcceptFamilyInvitation { family_id, node_id }` is called with no pending invitation stored for that pair
- **THEN** the call fails with `InvitationNotFound { family_id, node_id }`

### Requirement: Rejection and revocation work on expired invitations and archive them with terminal status

`RejectFamilyInvitation` (sent by the node controller) and `RevokeFamilyInvitation` (sent by the family owner) SHALL each:

- remove the pending invitation;
- archive a `PastFamilyInvitation` with status `Rejected { at: now }` or `Revoked { at: now }` respectively, under `((family_id, node_id), counter)` using the next free per-`(family, node)` archive slot;
- emit `family_invitation_rejected` or `family_invitation_revoked` respectively, with `family_id` and `node_id` attributes;
- fail with `InvitationNotFound { family_id, node_id }` if no pending invitation exists.

These two paths SHALL be the only ways to clear an expired invitation out of `pending_family_invitations` — the contract performs no background sweep of expired entries.

#### Scenario: Owner revokes a still-pending invitation
- **WHEN** family owner sends `RevokeFamilyInvitation { node_id }` for a node currently in their pending invitations
- **THEN** the pending entry is removed and the archive contains the invitation with status `Revoked { at: env.block.time.seconds() }`

#### Scenario: Node controller rejects a still-pending invitation
- **WHEN** node controller sends `RejectFamilyInvitation { family_id, node_id }` for a pending invitation
- **THEN** the pending entry is removed and the archive contains the invitation with status `Rejected { at: env.block.time.seconds() }`

#### Scenario: Expired invitations can still be rejected or revoked
- **WHEN** an invitation's `expires_at` is at or before `env.block.time.seconds()` and either `RejectFamilyInvitation` or `RevokeFamilyInvitation` is sent for it
- **THEN** the call succeeds, the pending entry is removed, and the archive records the appropriate terminal status

### Requirement: Leave and kick remove the membership and archive a past-member record

`ExecuteMsg::LeaveFamily { node_id }` SHALL require the sender to be the controller of `node_id` (failing with `SenderDoesntControlNode` otherwise). `ExecuteMsg::KickFromFamily { node_id }` SHALL require the sender to be the current owner of a family, and the node MUST be a member of *that* family — kicking a node belonging to a different family fails with `NodeNotMemberOfFamily { node_id, family_id }`; kicking a node that has no membership at all fails with `NodeNotInFamily { node_id }`. Both handlers SHALL share the same storage operation: remove `family_members[node_id]`, decrement `NodeFamily::members`, and archive a `PastFamilyMember { family_id, node_id, removed_at: env.block.time.seconds() }` under `((family_id, node_id), counter)` using the next free per-`(family, node)` slot. The respective events are `family_member_left` (carrying the leaver's `family_id` and `node_id`) and `family_member_kicked` (carrying the owner's `family_id` and the kicked `node_id`).

#### Scenario: Node controller leaves the family
- **WHEN** node controller sends `LeaveFamily { node_id }` for a node currently in family `F`
- **THEN** `family_members[node_id]` is removed, `F.members` is decremented, the archive contains a `PastFamilyMember` with `removed_at = env.block.time.seconds()`, and the response carries a `family_member_left` event

#### Scenario: Owner kicks a member of their family
- **WHEN** family owner sends `KickFromFamily { node_id }` for a member of their family
- **THEN** the same storage transition as a leave occurs and the response carries a `family_member_kicked` event

#### Scenario: Owner cannot kick a node belonging to another family
- **WHEN** family owner sends `KickFromFamily { node_id }` for a node whose membership is in a different family `G`
- **THEN** the call fails with `NodeNotMemberOfFamily { node_id, family_id: owned.id }` and storage is unchanged

#### Scenario: Repeated join/leave of the same family uses sequential archive counters
- **WHEN** node `n` joins, leaves, joins again, and leaves again the same family `F`
- **THEN** the archive contains `past_family_members[((F.id, n), 0)]` and `past_family_members[((F.id, n), 1)]` with the respective `removed_at` timestamps

### Requirement: Only the configured mixnet contract may invoke the unbonding callback

`ExecuteMsg::OnNymNodeUnbond { node_id }` SHALL be authorised solely by checking `info.sender == stored mixnet_contract_address`, failing with `UnauthorisedMixnetCallback { sender }` otherwise. There is no node-side or admin override.

#### Scenario: Mixnet contract triggers the callback
- **WHEN** the mixnet contract sends `OnNymNodeUnbond { node_id }`
- **THEN** the call proceeds and the cleanup (see next requirement) is applied

#### Scenario: Arbitrary sender is rejected
- **WHEN** any address other than the stored mixnet contract sends `OnNymNodeUnbond`
- **THEN** the call fails with `UnauthorisedMixnetCallback { sender }`

### Requirement: The unbonding callback is idempotent over membership and sweeps every pending invitation addressed to the node

A successful `OnNymNodeUnbond { node_id }` SHALL:

- if `family_members[node_id]` exists, remove the membership and archive a `PastFamilyMember` exactly as the `leave` / `kick` path does (decrementing the family's `members` count);
- if no such membership exists, leave membership state untouched (this is the common case — most unbonding nodes were never in a family);
- iterate every entry of `pending_family_invitations` keyed by `node_id` (via the `node` multi-index), remove each from the pending map, and archive each as `PastFamilyInvitation { invitation, status: Rejected { at: env.block.time.seconds() } }` using the next free per-`(family, node)` counter;
- emit a `family_node_unbond_cleanup` event with `node_id` attribute.

The auto-cleared invitations share the `Rejected` terminal state with invitations the node controller would have explicitly declined — `Revoked` is reserved for owner-side actions.

**Operational note (non-normative)**: like the disband sweep, the per-node invitation sweep iterates every pending invitation addressed to `node_id` and archives each in a single transaction. Gas cost therefore scales linearly with the number of outstanding invitations the unbonding node holds. If a node has accumulated an unusually large number of pending invitations, the operator-initiated unbond transaction on the mixnet contract may fail because *this* callback exceeds the per-tx gas limit. The fix is operator-side: explicitly `RejectFamilyInvitation` the outstanding invitations (in batches if needed) before retrying the unbond. There is no contract-side chunking, and the mixnet contract has no path to bypass or partial-apply the callback.

#### Scenario: Unbonding node with no family and no pending invitations is a no-op
- **WHEN** `OnNymNodeUnbond { node_id }` is invoked for a node with no membership and no pending invitations
- **THEN** the call succeeds, no storage is changed (apart from emitting the cleanup event), and no error is returned

#### Scenario: Unbonding node in a family loses its membership
- **WHEN** `OnNymNodeUnbond { node_id }` is invoked for a node currently in family `F`
- **THEN** `family_members[node_id]` is removed, `F.members` is decremented, and a `PastFamilyMember` is archived with `removed_at = env.block.time.seconds()`

#### Scenario: Unbonding node's pending invitations are archived as Rejected
- **WHEN** node `n` has pending invitations from families `F1` and `F2` and `OnNymNodeUnbond { node_id: n }` is invoked
- **THEN** both pending entries are removed and `past_family_invitations[((F1.id, n), …)]` and `past_family_invitations[((F2.id, n), …)]` each carry `status = Rejected { at: env.block.time.seconds() }`

### Requirement: Expired pending invitations remain in storage until explicitly cleared

The contract SHALL NOT run any background sweep of expired pending invitations. A `FamilyInvitation` whose `expires_at <= env.block.time.seconds()` SHALL remain in `pending_family_invitations` until either the family owner revokes it, the node controller rejects it, or the family is disbanded. Accept attempts against such entries MUST fail with `InvitationExpired`. Read queries SHALL surface a boolean `expired` flag (`now >= invitation.expires_at`) on each returned `PendingFamilyInvitationDetails` so callers can filter without doing the comparison themselves.

#### Scenario: Expired invitation is still listed by pending queries
- **WHEN** family `F` has a pending invitation for node `n` whose `expires_at` is in the past and `GetPendingInvitationsForFamilyPaged { family_id: F.id }` is queried
- **THEN** the result includes the entry with `expired = true`

#### Scenario: Expired invitation can be revoked or rejected but not accepted
- **WHEN** an expired invitation exists for `(F.id, n)`
- **THEN** `AcceptFamilyInvitation` fails with `InvitationExpired`, but `RevokeFamilyInvitation` (by `F`'s owner) and `RejectFamilyInvitation` (by `n`'s controller) both succeed and archive the invitation with the corresponding terminal status

### Requirement: Per-`(family, node)` archive slots use sequential counters starting at 0

Both `past_family_invitations` and `past_family_members` SHALL be keyed by `((family_id, node_id), counter: u64)`. The contract SHALL maintain explicit per-`(family, node)` counter maps (`past_family_invitation_counter`, `past_family_member_counter`), starting at `0` for each new pair and incrementing by `1` on each archival write. Counters MUST be independent across distinct `(family, node)` pairs and across the two archive types.

#### Scenario: First archive slot for a new pair is 0
- **WHEN** a node and family appear in either archive for the first time
- **THEN** the entry is keyed with `counter = 0`

#### Scenario: Distinct pairs have independent counters
- **WHEN** archive entries are written under keys `(F1, n)` and `(F2, n)` in any order
- **THEN** each pair's counter starts independently at `0`

### Requirement: GetFamilyMembership returns the family id a node belongs to, or None

`QueryMsg::GetFamilyMembership { node_id }` SHALL return `NodeFamilyMembershipResponse { node_id, family_id }` where `family_id` is `Some(NodeFamilyId)` iff `family_members[node_id]` is populated, and `None` otherwise. The query MUST NOT fail when `node_id` is unknown — it returns `family_id: None`. The query is the canonical way for route-selection consumers to check whether two node ids belong to the same family without scanning members.

#### Scenario: Member node returns its family id
- **WHEN** node `n` is currently a member of family `F` and `GetFamilyMembership { node_id: n }` is queried
- **THEN** the response carries `family_id = Some(F.id)`

#### Scenario: Non-member node returns None
- **WHEN** `GetFamilyMembership { node_id }` is queried for a node that has no membership record (never joined, or has since left/been kicked/unbonded)
- **THEN** the response carries `family_id = None` and `node_id` echoed back

### Requirement: Single-family-by-id, by-name, and by-owner queries return an `Option<NodeFamily>`

The contract SHALL expose three single-family lookups via `QueryMsg::GetFamilyById`, `GetFamilyByName`, and `GetFamilyByOwner`, each returning a response that echoes the queried key back to the caller for correlation and a `family: Option<NodeFamily>` field that is `None` when no match exists. `GetFamilyByName` MUST normalise the input via the same `normalise_family_name` function the create path uses before consulting the unique index. `GetFamilyByOwner` MUST bech32-validate the input.

#### Scenario: GetFamilyByName is invariant under input formatting
- **WHEN** family `F` exists with `name = "MyFamily"` and `GetFamilyByName { name: "my family" }` is queried
- **THEN** the response carries `family = Some(F)` (the lookup hits the unique normalised-name index)

#### Scenario: Missing match returns None
- **WHEN** any of the three single-family queries is called with a key that matches no family
- **THEN** the response carries `family = None` and echoes the queried key

#### Scenario: GetFamilyByOwner rejects an invalid bech32 address
- **WHEN** `GetFamilyByOwner { owner }` is called with a non-bech32 string
- **THEN** the call returns an error sourced from `Addr::validate`

### Requirement: Paginated queries use exclusive `start_after`, default limit 50, max limit 100

Every paginated query (`GetFamiliesPaged`, `GetFamilyMembersPaged`, `GetAllFamilyMembersPaged`, `GetPendingInvitationsForFamilyPaged`, `GetPendingInvitationsForNodePaged`, `GetAllPendingInvitationsPaged`, `GetPastInvitationsForFamilyPaged`, `GetPastInvitationsForNodePaged`, `GetAllPastInvitationsPaged`, `GetPastMembersForFamilyPaged`, `GetPastMembersForNodePaged`) SHALL accept `start_after: Option<Cursor>` (exclusive) and `limit: Option<u32>`. The default `limit` SHALL be `50` and SHALL be clamped to `100` as a hard cap. Results SHALL be in ascending cursor order. Each response SHALL include a `start_next_after: Option<Cursor>` derived from the last entry of the page, with `None` indicating the page is empty (the caller treats this as end-of-list).

Per-family / per-node paginated queries SHALL NOT verify that the supplied `family_id` or `node_id` corresponds to an existing entity — an unknown id simply yields an empty page.

#### Scenario: Limit is defaulted and clamped
- **WHEN** any paginated query is called with `limit = None`
- **THEN** at most 50 entries are returned
- **AND** when called with `limit = Some(10_000)` at most 100 entries are returned

#### Scenario: Empty page signals end-of-list
- **WHEN** a paginated query has no more entries past the supplied `start_after`
- **THEN** the response carries an empty entry list and `start_next_after = None`

#### Scenario: Unknown scope id yields an empty page rather than an error
- **WHEN** `GetFamilyMembersPaged { family_id: 999_999, .. }` is queried for a non-existent family id
- **THEN** the response carries an empty `members` list and `start_next_after = None` (no error is returned)

### Requirement: Pending-invitation queries stamp each entry with its expiry flag

Queries returning pending invitations (`GetPendingInvitation`, `GetPendingInvitationsForFamilyPaged`, `GetPendingInvitationsForNodePaged`, `GetAllPendingInvitationsPaged`) SHALL wrap each invitation in `PendingFamilyInvitationDetails { invitation, expired }` where `expired = env.block.time.seconds() >= invitation.expires_at`. Past-invitation and past-member queries do not carry this flag (they are terminal-state archives).

#### Scenario: Future-dated invitation is marked not expired
- **WHEN** a pending invitation has `expires_at = env.block.time.seconds() + 60` and any pending-invitation query returns it
- **THEN** the returned `PendingFamilyInvitationDetails.expired` is `false`

#### Scenario: Past-dated invitation is marked expired
- **WHEN** a pending invitation has `expires_at = env.block.time.seconds() - 1`
- **THEN** any pending-invitation query that returns it sets `expired = true`

### Requirement: Storage-key constants are part of the public contract

Every constant exported under `nym_node_families_contract_common::constants::storage_keys` SHALL be treated as part of the public contract surface. Off-chain indexers and migration tooling may depend on these exact byte strings. Changing the value of any existing constant — including renaming a namespace — SHALL be considered a breaking change for already-deployed contracts and SHALL be accompanied by a data migration via `queued_migrations`. Adding new constants for new storage maps is non-breaking. The current set covers the contract-level items (admin, config, mixnet contract address, family id counter), the primary maps (`families`, `node-family-members`, `invitations`, `past-invitations`, `past-family-member`), every secondary index (`__owner`, `__name`, `__family`, `__node`), and the per-`(family, node)` archive counters — refer to `constants.rs` for the authoritative list.

#### Scenario: Storage key constants are reachable from the common crate
- **WHEN** an off-chain consumer imports `nym_node_families_contract_common::constants::storage_keys`
- **THEN** it observes the full set of storage-key constants the contract uses, without taking a dependency on the contract crate itself

### Requirement: Emitted events form a stable public surface

Every event name and attribute key exported under `nym_node_families_contract_common::constants::events` SHALL be treated as part of the public contract surface. Each successful **state-mutating user-facing execute path** (every variant of `ExecuteMsg` except `UpdateConfig`, which is an admin handler that returns `Response::default()` without an event) SHALL emit exactly one event whose name and attribute keys come from these constants. Renaming an event name constant, renaming an attribute-key constant, or changing the set of attributes a given event carries SHALL be treated as breaking changes. Adding new constants for new events / attributes is non-breaking.

At the time of this spec the constant surface comprises (refer to `constants.rs` for the authoritative list):

- `family_creation` — attributes: `family_name`, `owner_address`, `family_id`, `paid_fee`
- `family_disband` — attributes: `family_id`, `owner_address`, `refunded_fee`
- `family_invitation` — attributes: `family_id`, `node_id`, `expires_at`
- `family_invitation_revoked` — attributes: `family_id`, `node_id`
- `family_invitation_accepted` — attributes: `family_id`, `node_id`
- `family_invitation_rejected` — attributes: `family_id`, `node_id`
- `family_member_left` — attributes: `family_id`, `node_id`
- `family_member_kicked` — attributes: `family_id`, `node_id`
- `family_node_unbond_cleanup` — attributes: `node_id`

**Note for indexer authors (non-normative)**: CosmWasm wraps user-emitted events in a `wasm-` prefix when they land in the tendermint event log. So `family_creation` appears as `wasm-family_creation` on the chain side, `family_invitation` as `wasm-family_invitation`, etc. The constants in `constants::events` are the *unprefixed* names — indexers querying the chain need to add the `wasm-` prefix themselves (or use the prefix-stripping helpers most Cosmos indexer SDKs already provide).

#### Scenario: Each successful state-mutating execute path emits exactly its named event
- **WHEN** any of the execute paths listed above succeeds (i.e. every `ExecuteMsg` variant other than `UpdateConfig`)
- **THEN** the response carries exactly one event whose `ty` equals the corresponding constant from `constants::events` and whose attributes include each of the listed keys

#### Scenario: UpdateConfig is the exception — no event is emitted
- **WHEN** the admin sends `ExecuteMsg::UpdateConfig` and the call succeeds
- **THEN** the response is `Response::default()` and carries no event (this is intentional — `UpdateConfig` is administrative metadata, not a tracked state transition)
