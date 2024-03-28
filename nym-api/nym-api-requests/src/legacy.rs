// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::mixnode::PendingMixNodeChanges;
use nym_mixnet_contract_common::{
    GatewayBond, LegacyMixLayer, MixNodeBond, MixNodeDetails, NodeId, NodeRewarding,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewayBondWithId {
    // we need to flatten it so that consumers of endpoints that returned `GatewayBond` wouldn't break
    #[serde(flatten)]
    pub bond: GatewayBond,
    pub node_id: NodeId,
}

impl Deref for LegacyGatewayBondWithId {
    type Target = GatewayBond;
    fn deref(&self) -> &Self::Target {
        &self.bond
    }
}

impl From<LegacyGatewayBondWithId> for GatewayBond {
    fn from(value: LegacyGatewayBondWithId) -> Self {
        value.bond
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyMixNodeBondWithLayer {
    // we need to flatten it so that consumers of endpoints that returned `MixNodeBond` wouldn't break
    #[serde(flatten)]
    pub bond: MixNodeBond,

    pub layer: LegacyMixLayer,
}

impl Deref for LegacyMixNodeBondWithLayer {
    type Target = MixNodeBond;
    fn deref(&self) -> &Self::Target {
        &self.bond
    }
}

impl From<LegacyMixNodeBondWithLayer> for MixNodeBond {
    fn from(value: LegacyMixNodeBondWithLayer) -> Self {
        value.bond
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LegacyMixNodeDetailsWithLayer {
    /// Basic bond information of this mixnode, such as owner address, original pledge, etc.
    pub bond_information: LegacyMixNodeBondWithLayer,

    /// Details used for computation of rewarding related data.
    pub rewarding_details: NodeRewarding,

    /// Adjustments to the mixnode that are ought to happen during future epoch transitions.
    #[serde(default)]
    pub pending_changes: PendingMixNodeChanges,
}

impl LegacyMixNodeDetailsWithLayer {
    pub fn mix_id(&self) -> NodeId {
        self.bond_information.mix_id
    }

    pub fn total_stake(&self) -> Decimal {
        self.rewarding_details.node_bond()
    }

    pub fn is_unbonding(&self) -> bool {
        self.bond_information.is_unbonding
    }
}

impl From<LegacyMixNodeDetailsWithLayer> for MixNodeDetails {
    fn from(value: LegacyMixNodeDetailsWithLayer) -> Self {
        MixNodeDetails {
            bond_information: value.bond_information.into(),
            rewarding_details: value.rewarding_details,
            pending_changes: value.pending_changes,
        }
    }
}
