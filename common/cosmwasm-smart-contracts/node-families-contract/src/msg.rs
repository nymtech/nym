// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    Config, GlobalPastFamilyInvitationCursor, NodeFamilyId, PastFamilyInvitationCursor,
    PastFamilyInvitationForNodeCursor, PastFamilyMemberCursor, PastFamilyMemberForNodeCursor,
};
use cosmwasm_schema::cw_serde;
use nym_mixnet_contract_common::NodeId;

#[cfg(feature = "schema")]
use crate::{
    AllPastFamilyInvitationsPagedResponse, FamiliesPagedResponse, FamilyMembersPagedResponse,
    NodeFamilyByNameResponse, NodeFamilyByOwnerResponse, NodeFamilyMembershipResponse,
    NodeFamilyResponse, PastFamilyInvitationsForNodePagedResponse,
    PastFamilyInvitationsPagedResponse, PastFamilyMembersForNodePagedResponse,
    PastFamilyMembersPagedResponse, PendingFamilyInvitationResponse,
    PendingFamilyInvitationsPagedResponse, PendingInvitationsForNodePagedResponse,
    PendingInvitationsPagedResponse,
};

pub use nym_contracts_common::Percent;

/// Message used to instantiate the node families contract.
#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,

    pub mixnet_contract_address: String,
}

/// Execute messages accepted by the contract.
#[cw_serde]
pub enum ExecuteMsg {
    //
}

/// Query messages accepted by the contract.
#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    /// Look up a single family by its id.
    #[cfg_attr(feature = "schema", returns(NodeFamilyResponse))]
    GetFamilyById { family_id: NodeFamilyId },

    /// Look up the (at most one) family owned by a given address.
    #[cfg_attr(feature = "schema", returns(NodeFamilyByOwnerResponse))]
    GetFamilyByOwner { owner: String },

    /// Look up a single family by its name. The lookup is normalised
    /// contract-side (lowercased, non-alphanumerics stripped), so equivalent
    /// inputs resolve to the same family.
    #[cfg_attr(feature = "schema", returns(NodeFamilyByNameResponse))]
    GetFamilyByName { name: String },

    #[cfg_attr(feature = "schema", returns(FamiliesPagedResponse))]
    GetFamiliesPaged {
        start_after: Option<NodeFamilyId>,
        limit: Option<u32>,
    },

    /// Look up which family — if any — a node currently belongs to.
    #[cfg_attr(feature = "schema", returns(NodeFamilyMembershipResponse))]
    GetFamilyMembership { node_id: NodeId },

    /// Page through every node currently in a given family.
    #[cfg_attr(feature = "schema", returns(FamilyMembersPagedResponse))]
    GetFamilyMembersPaged {
        family_id: NodeFamilyId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    },

    /// Look up the pending invitation for a specific `(family_id, node_id)`
    /// pair.
    #[cfg_attr(feature = "schema", returns(PendingFamilyInvitationResponse))]
    GetPendingInvitation {
        family_id: NodeFamilyId,
        node_id: NodeId,
    },

    /// Page through every pending invitation issued by a given family.
    #[cfg_attr(feature = "schema", returns(PendingFamilyInvitationsPagedResponse))]
    GetPendingInvitationsForFamilyPaged {
        family_id: NodeFamilyId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    },

    /// Page through every pending invitation issued for a given node.
    #[cfg_attr(feature = "schema", returns(PendingInvitationsForNodePagedResponse))]
    GetPendingInvitationsForNodePaged {
        node_id: NodeId,
        start_after: Option<NodeFamilyId>,
        limit: Option<u32>,
    },

    /// Page through every pending invitation across all families.
    #[cfg_attr(feature = "schema", returns(PendingInvitationsPagedResponse))]
    GetAllPendingInvitationsPaged {
        start_after: Option<(NodeFamilyId, NodeId)>,
        limit: Option<u32>,
    },

    /// Page through every archived (terminal-state) invitation issued by a
    /// given family.
    #[cfg_attr(feature = "schema", returns(PastFamilyInvitationsPagedResponse))]
    GetPastInvitationsForFamilyPaged {
        family_id: NodeFamilyId,
        start_after: Option<PastFamilyInvitationCursor>,
        limit: Option<u32>,
    },

    /// Page through every archived (terminal-state) invitation issued to a
    /// given node.
    #[cfg_attr(feature = "schema", returns(PastFamilyInvitationsForNodePagedResponse))]
    GetPastInvitationsForNodePaged {
        node_id: NodeId,
        start_after: Option<PastFamilyInvitationForNodeCursor>,
        limit: Option<u32>,
    },

    /// Page through every archived (terminal-state) invitation across all
    /// families.
    #[cfg_attr(feature = "schema", returns(AllPastFamilyInvitationsPagedResponse))]
    GetAllPastInvitationsPaged {
        start_after: Option<GlobalPastFamilyInvitationCursor>,
        limit: Option<u32>,
    },

    /// Page through every archived membership record for a given family
    /// (nodes that used to belong to it but have since been removed).
    #[cfg_attr(feature = "schema", returns(PastFamilyMembersPagedResponse))]
    GetPastMembersForFamilyPaged {
        family_id: NodeFamilyId,
        start_after: Option<PastFamilyMemberCursor>,
        limit: Option<u32>,
    },

    /// Page through every archived membership record for a given node
    /// (every family the node used to belong to but has since been removed
    /// from), across all families.
    #[cfg_attr(feature = "schema", returns(PastFamilyMembersForNodePagedResponse))]
    GetPastMembersForNodePaged {
        node_id: NodeId,
        start_after: Option<PastFamilyMemberForNodeCursor>,
        limit: Option<u32>,
    },
}

/// Message passed to the contract's `migrate` entry point.
#[cw_serde]
pub struct MigrateMsg {
    //
}
