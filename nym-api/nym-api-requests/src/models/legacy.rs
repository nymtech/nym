// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{CoinSchema, DeclaredRoles};
use crate::models::{NodePerformance, NymNodeData, StakeSaturation};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SemiSkimmedNode, SkimmedNode};
use cosmwasm_std::{Addr, Coin, Decimal};
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::rewarding::RewardEstimate;
use nym_mixnet_contract_common::{GatewayBond, Interval, MixNode, NodeId, RewardingParams};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;
use std::time::Duration;
use thiserror::Error;
use utoipa::{IntoParams, ToSchema};

#[deprecated]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayBondAnnotated {}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[deprecated]
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MixNodeBondAnnotated {}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewayBondWithId {
    #[serde(flatten)]
    pub bond: GatewayBond,
    pub node_id: NodeId,
}
