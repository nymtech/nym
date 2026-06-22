## ADDED Requirements

### Requirement: Owner's nym-node is auto-joined at family creation (NYM-1558)

The owner's bonded nym-node SHALL become the founding member of a newly created family with no separate invite step in the wallet. This is handled atomically by the node-families contract during `CreateFamily` (see `openspec/specs/node-families-contract/spec.md`): when the creating account controls a bonded, not-unbonding node that is not already in a family, the contract enrols it as the founding member (`members = 1`). The wallet therefore SHALL NOT perform a follow-up invite/accept sequence of its own — it simply creates the family and surfaces the resulting membership.

#### Scenario: Node auto-added after creation
- **WHEN** an account that controls a bonded, not-unbonding node creates a family
- **THEN** that node SHALL appear in the Joined rows of the members table immediately after the create transaction settles and the data refreshes
- **THEN** the wallet SHALL NOT have sent a separate invite to the owner's own node

#### Scenario: No auto-add when account has no bonded node
- **WHEN** the creating account does not control any bonded node
- **THEN** the family is created with no founding member and the members table shows no joined entries until the owner invites nodes

#### Scenario: Unbonding node is not auto-added
- **WHEN** the creating account controls a bonded node that is in the unbonding state
- **THEN** the family is created with no founding member (the contract skips enrolment)

#### Scenario: Owner can leave their own node from the family
- **WHEN** the owner's node was auto-enrolled at creation
- **THEN** the owner SHALL be able to remove/leave their own node via the standard leave/remove mechanism
- **THEN** the family SHALL continue to exist after the owner's node exits

## Notes

The original plan was for the wallet to chain `inviteToFamily` + `acceptFamilyInvitation` after `createFamily` and to show a "Your node {id} will be added automatically" hint in the create form. That approach was dropped in favour of doing the enrolment atomically in the contract, which removes the partial-failure window (no intermediate "stuck in Pending" state) and needs no extra Tauri round-trips from the wallet. The create form (`CreateFamilyForm`) therefore does not render an auto-add hint, and `CreateFamilyEntry.handleCreate` only calls `createFamily`.
