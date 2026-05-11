// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Coin;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct PendingFamilyInvitation {
    pub node_id: NodeId,

    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,

    pub expired: bool,
}

#[derive(Serialize, Deserialize)]
pub struct NodeFamilyMember {
    pub node_id: NodeId,

    #[serde(with = "time::serde::rfc3339")]
    pub joined_at: OffsetDateTime,
    pub stake: Coin,
    pub bond: Coin,
    pub delegations: Coin,
    pub delegators: usize,
}

#[derive(Serialize, Deserialize)]
pub struct NodeFamily {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub owner: String,

    #[serde(with = "humantime_serde")]
    pub average_node_age: Duration,

    pub total_stake: Coin,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    pub members: Vec<NodeFamilyMember>,

    pub pending_invitations: Vec<PendingFamilyInvitation>,
}
