# node-families-operator Specification

## Purpose
TBD - created by archiving change node-families-wallet. Update Purpose after archive.
## Requirements
### Requirement: Node operator SHALL view family invites per node

_Design: canonical `2474:2063` (Member, pending invite); ref `1861:1349` (SECTION 5 · Incoming Invite popups · NYM-1216)._

The wallet SHALL display, in the Family tab, the pending family invitations addressed to each node the operator controls. When the operator controls multiple nodes, invitations SHALL be shown separately per node. Each invitation SHALL show the family name, the inviting family owner, and the expiry (TTL). Invitations whose contract `expired` flag is true SHALL be shown as **expired** and SHALL NOT be actionable (no accept/reject). Invitations are sourced from `GetPendingInvitationsForNodePaged` per controlled node.

#### Scenario: Active invite is shown with details
- **WHEN** a controlled node has a pending, not-yet-expired invitation
- **THEN** the wallet shows the family name, inviting owner, and expiry, and offers accept/reject actions

#### Scenario: Expired invite is shown as non-actionable
- **WHEN** a pending invitation's `expired` flag is true
- **THEN** the wallet shows it as expired and offers no accept/reject actions

#### Scenario: Multiple nodes show their invites separately
- **WHEN** the operator controls more than one node, each with different invitations
- **THEN** the wallet groups invitations under their respective node and shows each node's distinct invite state

### Requirement: Node operator SHALL accept an invite

_Design: ref `1861:1349` (SECTION 5 · accept · NYM-1218), incl. on-chain-consequences confirm; canonical `2474:2063` (Member, pending invite)._

The wallet SHALL let the operator accept a pending, not-yet-expired invitation from the invite view, triggering `AcceptFamilyInvitation { family_id, node_id }`. On success the wallet MUST show a confirmation and the node MUST appear as **Joined** in the family member list. In V1 acceptance records membership only; the family owner gains no control over the node itself (owner-acts-for-node is V2 per NYM-1217). Accepting an expired invitation MUST be prevented (`InvitationExpired`).

#### Scenario: Successful acceptance
- **WHEN** the operator accepts a not-yet-expired invitation for a node they control
- **THEN** `AcceptFamilyInvitation` is triggered, a confirmation is shown, and the node is reflected as Joined

#### Scenario: Expired invitation cannot be accepted
- **WHEN** the operator attempts to accept an invitation whose `expired` flag is true
- **THEN** the wallet prevents acceptance and surfaces an expired error

### Requirement: Node operator SHALL reject an invite

_Design: ref `1861:1349` (SECTION 5 · reject · NYM-1217/1218)._

The wallet SHALL let the operator reject a pending invitation from the invite view, behind a confirmation prompt, triggering `RejectFamilyInvitation { family_id, node_id }`. After rejection the invitation MUST no longer appear in the operator's pending list, and the node MUST appear under **Rejected** in the family member list.

#### Scenario: Successful rejection
- **WHEN** the operator rejects a pending invitation and confirms the prompt
- **THEN** `RejectFamilyInvitation` is triggered and the invitation is removed from the pending list

#### Scenario: Rejected invite is no longer shown
- **WHEN** an invitation has been rejected
- **THEN** it is not shown again in the operator's pending invite list and the node shows as Rejected in the family member list

### Requirement: Node operator SHALL leave a family

_Design: ref `1861:1711` (SECTION 6 · Leave family · NYM-1219); canonical `2474:2134` (Member, active)._

The wallet SHALL let an operator whose node is a member of a family leave it voluntarily from the Family tab, behind a confirmation prompt, triggering `LeaveFamily { node_id }`. After leaving, the node MUST be removed from the family member list (shown as Removed) and the operator MUST subsequently be able to receive and accept invitations from other families.

#### Scenario: Successful leave
- **WHEN** the operator leaves a family and confirms the prompt
- **THEN** `LeaveFamily` is triggered and the node is removed from the family member list

#### Scenario: Node can join another family after leaving
- **WHEN** a node has left a family
- **THEN** the operator can receive and accept invitations from other families for that node

