// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::AVERAGE_BLOCK_TIME;
use cosmwasm_std::Coin;
use nym_api_requests::models::node_families::{
    NodeFamily, NodeFamilyMember, NodeStakeInformation, PendingFamilyInvitation,
};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails};
use nym_node_families_contract_common::{
    FamilyMemberRecord, NodeFamilyId, PendingFamilyInvitationDetails,
};
use nym_validator_client::nyxd::contract_traits::PinnedNodeFamiliesQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tendermint::block::Height;
use time::OffsetDateTime;
use tracing::error;

/// A single family member, joining the contract membership record with the node details read at
/// the same height (bond height + stake).
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CachedFamilyMember {
    pub(crate) node_id: NodeId,

    /// Block-time at which the node joined the family.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) joined_at: OffsetDateTime,

    /// Bonding height from the mixnet contract; `None` if the node is not bonded at the
    /// snapshot height.
    pub(crate) bonding_height: Option<u64>,

    /// Stake/bond/delegation snapshot; `None` if the node is not bonded at the snapshot height.
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

/// Pending invitation entry for a family.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CachedFamilyInvitation {
    /// Node the invitation is addressed to.
    pub(crate) node_id: NodeId,

    /// Block-time after which the invitation can no longer be accepted.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) expires_at: OffsetDateTime,
}

impl From<PendingFamilyInvitationDetails> for CachedFamilyInvitation {
    fn from(invitation: PendingFamilyInvitationDetails) -> Self {
        CachedFamilyInvitation {
            node_id: invitation.invitation.node_id,
            expires_at: OffsetDateTime::from_unix_timestamp(
                invitation.invitation.expires_at as i64,
            )
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

/// Family record with its current members, pending invitations and aggregated stats
/// (`average_node_age`, `total_stake`).
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CachedFamily {
    /// Unique family identifier assigned by the contract.
    pub(crate) id: NodeFamilyId,

    /// Display name (canonical form — see `normalise_family_name`).
    pub(crate) name: String,

    /// Free-form family description.
    pub(crate) description: String,

    /// Owner address (cosmos `Addr` rendered as a string).
    pub(crate) owner: String,

    /// Average age of the members, counted in blocks since their bonding height.
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

/// The node families contract at a snapshot height, joined with the node details read there.
#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct NodeFamiliesData {
    /// Every family known to the contract, keyed by family id. `BTreeMap` so
    /// iteration order is deterministic (stable list endpoint output,
    /// deterministic on-disk bincode).
    pub(crate) families: BTreeMap<NodeFamilyId, CachedFamily>,

    /// Secondary index: member node id → family id. Built alongside
    /// `families` during the join; lets `by-node` lookups avoid an O(families
    /// × members) scan.
    pub(crate) family_by_member: HashMap<NodeId, NodeFamilyId>,
}

impl NodeFamiliesData {
    /// Every family at `height`, each query pinned there, joined with `nym_nodes`, the node
    /// details read at that height.
    pub(crate) async fn read<C>(
        client: &C,
        height: Height,
        nym_nodes: &[NymNodeDetails],
    ) -> Result<Self, NyxdError>
    where
        C: PinnedNodeFamiliesQueryClient + Sync,
    {
        let families = client.get_all_families_at_height(height).await?;
        let members = client.get_all_family_members_at_height(height).await?;
        let invitations = client.get_all_pending_invitations_at_height(height).await?;
        Ok(Self::join(
            height,
            families,
            members,
            invitations,
            nym_nodes,
        ))
    }

    fn join(
        height: Height,
        families: Vec<nym_node_families_contract_common::NodeFamily>,
        members: Vec<FamilyMemberRecord>,
        invitations: Vec<PendingFamilyInvitationDetails>,
        nym_nodes: &[NymNodeDetails],
    ) -> Self {
        let nym_nodes = nym_nodes
            .iter()
            .map(|node| (node.node_id(), node))
            .collect::<HashMap<_, _>>();

        let mut families: HashMap<_, CachedFamilyBuilder> = families
            .into_iter()
            .map(|family| (family.id, family.into()))
            .collect();
        let mut family_by_member = HashMap::new();

        for member in members {
            let family_id = member.membership.family_id;
            let node_id = member.node_id;
            let Some(family) = families.get_mut(&family_id) else {
                error!(
                    "node {node_id} belongs to family {family_id}, but this family does not exist!"
                );
                continue;
            };
            let node_information = nym_nodes.get(&node_id).copied();
            family
                .members
                .push(CachedFamilyMember::new(member, node_information));
            family_by_member.insert(node_id, family_id);
        }

        for invitation in invitations {
            let family_id = invitation.invitation.family_id;
            let node_id = invitation.invitation.node_id;
            let Some(family) = families.get_mut(&family_id) else {
                error!(
                    "node {node_id} has been invited to family {family_id}, but this family does not exist!",
                );
                continue;
            };
            family.pending_invitations.push(invitation.into());
        }

        let families = families
            .into_values()
            .map(|family| {
                let average_node_age = average_node_age(&family.members, height);
                let built = family.build(average_node_age);
                (built.id, built)
            })
            .collect();

        NodeFamiliesData {
            families,
            family_by_member,
        }
    }
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

    /// Members accumulated as the join iterates the contract response.
    pub(crate) members: Vec<CachedFamilyMember>,

    /// Pending invitations accumulated as the join iterates the contract
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

/// The mean age of the members bonded at `height`, each counted in blocks since its bonding
/// height, in whole seconds.
fn average_node_age(members: &[CachedFamilyMember], height: Height) -> Duration {
    let ages = members
        .iter()
        .filter_map(|member| member.bonding_height)
        .map(|bonding_height| {
            AVERAGE_BLOCK_TIME * height.value().saturating_sub(bonding_height) as u32
        })
        .collect::<Vec<_>>();
    if ages.is_empty() {
        return Duration::ZERO;
    }
    let mean = ages.iter().sum::<Duration>() / ages.len() as u32;
    Duration::from_secs(mean.as_secs())
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;

    /// Families data holding one family with every field and `Option` set.
    pub(crate) fn populated_families_data() -> NodeFamiliesData {
        let stake = Coin::new(1_000u128, "unym");
        let family = CachedFamily {
            id: 1,
            name: "alpha".to_string(),
            description: "alpha description".to_string(),
            owner: "n1owner1".to_string(),
            average_node_age: Duration::from_secs(568_700),
            total_stake: Some(stake.clone()),
            created_at: OffsetDateTime::UNIX_EPOCH,
            members: vec![CachedFamilyMember {
                node_id: 10,
                joined_at: OffsetDateTime::UNIX_EPOCH,
                bonding_height: Some(26_000_000),
                node_stake_information: Some(NodeStakeInformation {
                    stake: stake.clone(),
                    bond: stake,
                    delegations: Coin::new(0u128, "unym"),
                    delegators: 0,
                }),
            }],
            pending_invitations: vec![CachedFamilyInvitation {
                node_id: 12,
                expires_at: OffsetDateTime::UNIX_EPOCH,
            }],
        };

        NodeFamiliesData {
            families: BTreeMap::from([(1, family)]),
            family_by_member: HashMap::from([(10, 1)]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member_bonded_at(bonding_height: u64) -> CachedFamilyMember {
        CachedFamilyMember {
            node_id: 1,
            joined_at: OffsetDateTime::UNIX_EPOCH,
            bonding_height: Some(bonding_height),
            node_stake_information: None,
        }
    }

    #[test]
    fn a_member_is_aged_by_the_blocks_since_its_bond() {
        let members = [member_bonded_at(26_000_000)];

        assert_eq!(
            average_node_age(&members, Height::from(26_100_000u32)),
            Duration::from_secs(568_700)
        );
    }
}
