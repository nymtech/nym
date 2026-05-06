// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Storage key constants used by the node families contract.
///
/// They are kept in the common crate so that off-chain tooling (indexers, migration
/// scripts) can reference them without depending on the contract crate itself.
/// Changing any of these values is a breaking change for already-deployed contracts.
pub mod storage_keys {
    /// `Item<Addr>`: address of the mixnet contract used to validate node existence.
    pub const MIXNET_CONTRACT_ADDRESS: &str = "mixnet-contract-address";

    /// `Item<Config>`: runtime configuration (fees, length limits) set at instantiation.
    pub const CONFIG: &str = "config";

    /// `Admin` (cw-controllers): admin allowed to perform privileged operations.
    pub const CONTRACT_ADMIN: &str = "contract-admin";
    /// `Item<NodeFamilyId>`: monotonically increasing id counter for new families.
    pub const NODE_FAMILY_ID_COUNTER: &str = "node-family-id-counter";
    /// Primary namespace for the current family-members `IndexedMap`,
    /// keyed by `NodeId` with value [`crate::FamilyMembership`].
    pub const NODE_FAMILY_MEMBERS: &str = "node-family-members";
    /// Multi-index over current family members keyed by family id —
    /// enables paginated listing of all nodes in a given family.
    pub const NODE_FAMILY_MEMBERS_FAMILY_IDX_NAMESPACE: &str = "node-family-members__family";

    /// Primary namespace for the families `IndexedMap`.
    pub const FAMILIES_NAMESPACE: &str = "families";
    /// Secondary unique index keyed by `owner` (one family per owner).
    pub const FAMILIES_OWNER_IDX_NAMESPACE: &str = "families__owner";
    /// Secondary unique index keyed by `name` (family names are globally unique).
    pub const FAMILIES_NAME_IDX_NAMESPACE: &str = "families__name";

    /// Primary namespace for the pending invitations `IndexedMap`.
    pub const INVITATIONS_NAMESPACE: &str = "invitations";
    /// Multi-index over pending invitations keyed by family id.
    pub const INVITATIONS_FAMILY_IDX_NAMESPACE: &str = "invitations__family";
    /// Multi-index over pending invitations keyed by node id
    /// (a node can be invited to multiple families simultaneously).
    pub const INVITATIONS_NODE_IDX_NAMESPACE: &str = "invitations__node";

    /// Primary namespace for the archived (accepted/rejected/revoked) invitations `IndexedMap`.
    pub const PAST_INVITATIONS_NAMESPACE: &str = "past-invitations";
    /// Multi-index over past invitations keyed by family id.
    pub const PAST_INVITATIONS_FAMILY_IDX_NAMESPACE: &str = "past-invitations__family";
    /// Multi-index over past invitations keyed by node id.
    pub const PAST_INVITATIONS_NODE_IDX_NAMESPACE: &str = "past-invitations__node";
    /// `Map<(NodeFamilyId, NodeId), u64>`: per-`(family, node)` counter used to
    /// disambiguate repeat archive entries (a node can be invited and have the
    /// invitation reach a terminal state more than once).
    pub const PAST_INVITATIONS_COUNTER_NAMESPACE: &str = "past-invitations-counter";

    /// Primary namespace for the past-members `IndexedMap`.
    pub const PAST_FAMILY_MEMBER_NAMESPACE: &str = "past-family-member";
    /// Multi-index over past members keyed by family id.
    pub const PAST_FAMILY_MEMBER_FAMILY_IDX_NAMESPACE: &str = "past-family-member__family";
    /// Multi-index over past members keyed by node id.
    pub const PAST_FAMILY_MEMBER_NODE_IDX_NAMESPACE: &str = "past-family-member__node";
    /// `Map<(NodeFamilyId, NodeId), u64>`: per-`(family, node)` counter used to
    /// disambiguate repeat past-membership entries (a node can join and leave
    /// the same family more than once).
    pub const PAST_FAMILY_MEMBER_COUNTER_NAMESPACE: &str = "past-family-member-counter";
}

pub mod events {
    pub const FAMILY_CREATION_EVENT_NAME: &str = "family_creation";
    pub const FAMILY_CREATION_EVENT_FAMILY_NAME: &str = "family_name";
    pub const FAMILY_CREATION_EVENT_OWNER_ADDRESS: &str = "owner_address";
    pub const FAMILY_CREATION_EVENT_FAMILY_ID: &str = "family_id";

    pub const FAMILY_DISBAND_EVENT_NAME: &str = "family_disband";
    pub const FAMILY_DISBAND_EVENT_FAMILY_ID: &str = "family_id";
    pub const FAMILY_DISBAND_EVENT_OWNER_ADDRESS: &str = "owner_address";
    pub const FAMILY_DISBAND_EVENT_REFUNDED_FEE: &str = "refunded_fee";
}
