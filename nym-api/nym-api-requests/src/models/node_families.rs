// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Coin;
use nym_mixnet_contract_common::{NodeId, NodeRewarding};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;

/// Pending family invitation as exposed by the nym-api node-families endpoints.
#[derive(Serialize, Deserialize)]
pub struct PendingFamilyInvitation {
    /// Node the invitation is addressed to.
    pub node_id: NodeId,

    /// Block-time after which the invitation can no longer be accepted.
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

/// Per-node stake snapshot derived from the mixnet contract's rewarding state.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeStakeInformation {
    /// Operator bond + all delegations, with accrued rewards applied.
    pub stake: Coin,

    /// Operator pledge component of `stake`.
    pub bond: Coin,

    /// Delegations component of `stake`.
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
#[derive(Serialize, Deserialize)]
pub struct NodeFamilyMember {
    pub node_id: NodeId,

    /// Block-time at which the node joined the family.
    #[serde(with = "time::serde::rfc3339")]
    pub joined_at: OffsetDateTime,

    /// Stake/bond/delegation snapshot; `None` if the node was not in the
    /// mixnet-contract cache at refresh time.
    pub stake_information: Option<NodeStakeInformation>,
}

/// Family view as exposed by the nym-api node-families endpoints, carrying
/// current members, pending invitations and aggregated stats.
#[derive(Serialize, Deserialize)]
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
    pub average_node_age: Duration,

    /// Sum of member stakes; `None` when no member has reportable stake.
    pub total_stake: Option<Coin>,

    /// Block-time the family was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    /// Current members of the family.
    pub members: Vec<NodeFamilyMember>,

    /// Outstanding invitations issued by the family owner.
    pub pending_invitations: Vec<PendingFamilyInvitation>,
}
