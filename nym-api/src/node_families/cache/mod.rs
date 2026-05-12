// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Coin;
use nym_api_requests::models::node_families::{
    NodeFamily, NodeFamilyMember, NodeStakeInformation, PendingFamilyInvitation,
};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_node_families_contract_common::{FamilyMemberRecord, NodeFamilyId};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) mod refresher;

/// Cached view of a single family member, joining the contract membership
/// record with mixnet-contract node details (bond height + stake).
#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamilyMember {
    pub(crate) node_id: NodeId,

    /// Block-time at which the node joined the family.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) joined_at: OffsetDateTime,

    /// Bonding height from the mixnet contract; `None` if the node is no
    /// longer in the mixnet cache snapshot.
    pub(crate) bonding_height: Option<u64>,

    /// Stake/bond/delegation snapshot; `None` if the node is no longer in the
    /// mixnet cache snapshot.
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

/// Cached pending invitation entry for a family.
#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamilyInvitation {
    /// Node the invitation is addressed to.
    pub(crate) node_id: NodeId,

    /// Block-time after which the invitation can no longer be accepted.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) expires_at: OffsetDateTime,
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
        }
    }
}

/// Cached family record with its current members, pending invitations and
/// aggregated stats (`average_node_age`, `total_stake`).
#[derive(Serialize, Deserialize)]
pub(crate) struct CachedFamily {
    /// Unique family identifier assigned by the contract.
    pub(crate) id: NodeFamilyId,

    /// Display name (canonical form — see `normalise_family_name`).
    pub(crate) name: String,

    /// Free-form family description.
    pub(crate) description: String,

    /// Owner address (cosmos `Addr` rendered as a string).
    pub(crate) owner: String,

    /// Average age of members, approximated from the mean bonding height.
    #[serde(with = "humantime_serde")]
    pub(crate) average_node_age: Duration,

    /// Sum of member stakes; `None` when no member has reportable stake (e.g.
    /// all bonds unbonding).
    pub(crate) total_stake: Option<Coin>,

    /// Block-time the family was created.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,

    /// Current members of the family.
    pub(crate) members: Vec<CachedFamilyMember>,

    /// Outstanding invitations issued by the family owner.
    pub(crate) pending_invitations: Vec<CachedFamilyInvitation>,
}

/// Full nym-api node-families cache snapshot — combined families-contract
/// state plus mixnet-contract stake/bond information.
#[derive(Serialize, Deserialize)]
pub(crate) struct NodeFamiliesCacheData {
    /// Every family known to the contract, with members and pending invitations.
    pub(crate) families: Vec<CachedFamily>,
}

/// Intermediate accumulator used while folding contract data into a
/// [`CachedFamily`]; finalised via [`Self::build`] once `average_node_age` is
/// known.
pub(crate) struct CachedFamilyBuilder {
    /// Unique family identifier assigned by the contract.
    pub(crate) id: NodeFamilyId,

    /// Display name (canonical form — see `normalise_family_name`).
    pub(crate) name: String,

    /// Free-form family description.
    pub(crate) description: String,

    /// Owner address (cosmos `Addr` rendered as a string).
    pub(crate) owner: String,

    /// Block-time the family was created.
    pub(crate) created_at: OffsetDateTime,

    /// Members accumulated as the refresher iterates the contract response.
    pub(crate) members: Vec<CachedFamilyMember>,

    /// Pending invitations accumulated as the refresher iterates the contract
    /// response.
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

    /// Sum the per-member stake into a single family total. Returns `None` if
    /// no member has a known stake.
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

impl From<&CachedFamilyInvitation> for PendingFamilyInvitation {
    fn from(value: &CachedFamilyInvitation) -> Self {
        PendingFamilyInvitation {
            node_id: value.node_id,
            expires_at: value.expires_at,
        }
    }
}

impl From<&CachedFamilyMember> for NodeFamilyMember {
    fn from(value: &CachedFamilyMember) -> Self {
        NodeFamilyMember {
            node_id: value.node_id,
            joined_at: value.joined_at,
            stake_information: value.node_stake_information.clone(),
        }
    }
}

impl From<&CachedFamily> for NodeFamily {
    fn from(value: &CachedFamily) -> Self {
        NodeFamily {
            id: value.id,
            name: value.name.clone(),
            description: value.description.clone(),
            owner: value.owner.clone(),
            average_node_age: value.average_node_age,
            total_stake: value.total_stake.clone(),
            created_at: value.created_at,
            members: value.members.iter().map(Into::into).collect(),
            pending_invitations: value.pending_invitations.iter().map(Into::into).collect(),
        }
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
