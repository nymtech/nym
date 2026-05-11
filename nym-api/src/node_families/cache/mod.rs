// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Coin;
use nym_mixnet_contract_common::{NodeId, NodeRewarding, NymNodeDetails};
use nym_node_families_contract_common::{FamilyMemberRecord, NodeFamilyId};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) mod refresher;

#[derive(Serialize, Deserialize)]
pub(crate) struct NodeStakeInformation {
    pub stake: Coin,
    pub bond: Coin,
    pub delegations: Coin,
    pub delegators: usize,
}

impl From<&NodeRewarding> for NodeStakeInformation {
    fn from(rewarding: &NodeRewarding) -> Self {
        let denom = &rewarding.cost_params.interval_operating_cost.denom;

        let bond = rewarding.operator_pledge_with_reward(denom);
        let delegations = rewarding.delegations_with_reward(denom);
        let mut stake = bond.clone();
        stake.amount += delegations.amount;

        Self {
            stake,
            bond,
            delegations,
            delegators: rewarding.unique_delegations as usize,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamilyMember {
    pub(crate) node_id: NodeId,

    #[serde(with = "time::serde::rfc3339")]
    pub(crate) joined_at: OffsetDateTime,

    pub(crate) bonding_height: Option<u64>,

    pub(crate) node_stake_information: Option<NodeStakeInformation>,
}

impl CachedFamilyMember {
    pub(crate) fn new(
        record: FamilyMemberRecord,
        node_information: Option<&NymNodeDetails>,
    ) -> Self {
        CachedFamilyMember {
            node_id: record.node_id,
            joined_at: OffsetDateTime::from_unix_timestamp(record.membership.joined_at as i64)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            bonding_height: node_information.map(|n| n.bond_information.bonding_height),
            node_stake_information: node_information.map(|n| (&n.rewarding_details).into()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamilyInvitation {
    pub(crate) node_id: NodeId,

    #[serde(with = "time::serde::rfc3339")]
    pub(crate) expires_at: OffsetDateTime,

    pub(crate) expired: bool,
}

impl From<nym_node_families_contract_common::PendingFamilyInvitationDetails>
    for CachedFamilyInvitation
{
    fn from(invitation: nym_node_families_contract_common::PendingFamilyInvitationDetails) -> Self {
        CachedFamilyInvitation {
            node_id: invitation.invitation.node_id,
            expires_at: OffsetDateTime::from_unix_timestamp(
                invitation.invitation.expires_at as i64,
            )
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            expired: invitation.expired,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamily {
    pub(crate) id: NodeFamilyId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) owner: String,

    #[serde(with = "humantime_serde")]
    pub(crate) average_node_age: Duration,
    pub(crate) total_stake: Option<Coin>,

    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,
    pub(crate) members: Vec<CachedFamilyMember>,

    pub(crate) pending_invitations: Vec<CachedFamilyInvitation>,
}

// families contract + mixnet contract combined
#[derive(Serialize, Deserialize)]
pub(crate) struct NodeFamiliesCacheData {
    pub(crate) families: Vec<CachedFamily>,
}

pub(crate) struct CachedFamilyBuilder {
    pub(crate) id: NodeFamilyId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) owner: String,

    pub(crate) created_at: OffsetDateTime,
    pub(crate) members: Vec<CachedFamilyMember>,
    pub(crate) pending_invitations: Vec<CachedFamilyInvitation>,
}

impl CachedFamilyBuilder {
    pub(crate) fn build(self, average_node_age: Duration) -> CachedFamily {
        let total_stake = self.total_family_stake();

        CachedFamily {
            id: self.id,
            name: self.name,
            description: self.description,
            owner: self.owner,
            created_at: self.created_at,
            members: self.members,
            pending_invitations: self.pending_invitations,
            total_stake,
            average_node_age,
        }
    }

    pub(crate) fn total_family_stake(&self) -> Option<Coin> {
        self.members
            .iter()
            .filter_map(|m| m.node_stake_information.as_ref().map(|s| s.stake.clone()))
            .reduce(|acc, e| {
                let mut updated = acc;
                updated.amount += e.amount;
                updated
            })
    }
}

// initial conversion with empty details
impl From<nym_node_families_contract_common::NodeFamily> for CachedFamilyBuilder {
    fn from(value: nym_node_families_contract_common::NodeFamily) -> Self {
        CachedFamilyBuilder {
            id: value.id,
            name: value.name,
            description: value.description,
            owner: value.owner.to_string(),
            created_at: OffsetDateTime::from_unix_timestamp(value.created_at as i64)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            members: Vec::new(),
            pending_invitations: Vec::new(),
        }
    }
}
