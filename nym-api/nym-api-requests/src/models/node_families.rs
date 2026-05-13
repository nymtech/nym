// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::CoinSchema;
use cosmwasm_std::Coin;
use nym_mixnet_contract_common::{NodeId, NodeRewarding};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Pending family invitation as exposed by the nym-api node-families endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingFamilyInvitation {
    /// Node the invitation is addressed to.
    pub node_id: NodeId,

    /// Block-time after which the invitation can no longer be accepted.
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub expires_at: OffsetDateTime,
}

/// Per-node stake snapshot derived from the mixnet contract's rewarding state.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeStakeInformation {
    /// Operator bond + all delegations, with accrued rewards applied.
    #[schema(value_type = CoinSchema)]
    pub stake: Coin,

    /// Operator pledge component of `stake`.
    #[schema(value_type = CoinSchema)]
    pub bond: Coin,

    /// Delegations component of `stake`.
    #[schema(value_type = CoinSchema)]
    pub delegations: Coin,

    /// Number of unique delegators backing this node.
    pub delegators: usize,
}

impl From<&NodeRewarding> for NodeStakeInformation {
    fn from(rewarding: &NodeRewarding) -> Self {
        let denom = &rewarding.cost_params.interval_operating_cost.denom;

        let bond = rewarding.operator_pledge_with_reward(denom);
        let delegations = rewarding.delegations_with_reward(denom);
        let mut stake = bond.clone();
        stake.amount += delegations.amount;

        NodeStakeInformation {
            stake,
            bond,
            delegations,
            delegators: rewarding.unique_delegations as usize,
        }
    }
}

/// Family member view as exposed by the nym-api node-families endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeFamilyMember {
    pub node_id: NodeId,

    /// Block-time at which the node joined the family.
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub joined_at: OffsetDateTime,

    /// Stake/bond/delegation snapshot; `None` if the node was not in the
    /// mixnet-contract cache at refresh time.
    pub stake_information: Option<NodeStakeInformation>,
}

/// Family view as exposed by the nym-api node-families endpoints, carrying
/// current members, pending invitations and aggregated stats.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeFamily {
    /// Unique family identifier assigned by the contract.
    pub id: u32,

    /// Display name (canonical form — see `normalise_family_name`).
    pub name: String,

    /// Free-form family description.
    pub description: String,

    /// Owner address (cosmos `Addr` rendered as a string).
    pub owner: String,

    /// Time-weighted average age of members.
    #[serde(with = "humantime_serde")]
    #[schema(value_type = String)]
    pub average_node_age: Duration,

    /// Sum of member stakes; `None` when no member has reportable stake.
    #[schema(value_type = Option<CoinSchema>)]
    pub total_stake: Option<Coin>,

    /// Block-time the family was created.
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub created_at: OffsetDateTime,

    /// Current members of the family.
    pub members: Vec<NodeFamilyMember>,

    /// Outstanding invitations issued by the family owner.
    pub pending_invitations: Vec<PendingFamilyInvitation>,
}

/// Response wrapper for endpoints that look up a single family. `family` is
/// `None` when the lookup did not match (rather than returning a 404).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeFamilyResponse {
    pub family: Option<NodeFamily>,
}

/// Response wrapper for endpoints that look up the family a given node
/// belongs to. `family` is `None` when the node is not currently a member of
/// any cached family.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeFamilyForNodeResponse {
    /// The node the lookup was performed for.
    pub node_id: NodeId,

    /// The family this node belongs to, if any.
    pub family: Option<NodeFamily>,
}
