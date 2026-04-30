// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeFamilyId;
use cosmwasm_schema::cw_serde;
use nym_mixnet_contract_common::NodeId;

#[cfg(feature = "schema")]
use crate::{
    FamiliesPagedResponse, FamilyMembersPagedResponse, NodeFamilyMembershipResponse,
    NodeFamilyResponse, PendingFamilyInvitationResponse, PendingFamilyInvitationsPagedResponse,
    PendingInvitationsPagedResponse,
};

/// Message used to instantiate the node families contract.
#[cw_serde]
pub struct InstantiateMsg {
    //
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

    /// Page through every pending invitation across all families.
    #[cfg_attr(feature = "schema", returns(PendingInvitationsPagedResponse))]
    GetAllPendingInvitationsPaged {
        start_after: Option<(NodeFamilyId, NodeId)>,
        limit: Option<u32>,
    },
}

/// Message passed to the contract's `migrate` entry point.
#[cw_serde]
pub struct MigrateMsg {
    //
}
