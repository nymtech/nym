// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{CoinSchema, DeclaredRoles};
use crate::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct LegacyDescribedGateway {
    pub bond: GatewayBond,
    pub self_described: Option<NymNodeData>,
}

impl From<LegacyGatewayBondWithId> for LegacyDescribedGateway {
    fn from(bond: LegacyGatewayBondWithId) -> Self {
        LegacyDescribedGateway {
            bond: bond.bond,
            self_described: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct LegacyDescribedMixNode {
    pub bond: LegacyMixNodeBondWithLayer,
    pub self_described: Option<NymNodeData>,
}

impl From<LegacyMixNodeBondWithLayer> for LegacyDescribedMixNode {
    fn from(bond: LegacyMixNodeBondWithLayer) -> Self {
        LegacyDescribedMixNode {
            bond,
            self_described: None,
        }
    }
}

#[deprecated]
#[derive(Clone, Serialize, schemars::JsonSchema, ToSchema)]
pub struct InclusionProbability {
    #[schema(value_type = u32)]
    pub mix_id: NodeId,
    pub in_active: f64,
    pub in_reserve: f64,
}

#[deprecated]
#[derive(Clone, Serialize, schemars::JsonSchema, ToSchema)]
pub struct AllInclusionProbabilitiesResponse {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
    pub as_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/SelectionChance.ts"
    )
)]
#[deprecated]
pub enum SelectionChance {
    High,
    Good,
    Low,
}

impl From<f64> for SelectionChance {
    fn from(p: f64) -> SelectionChance {
        match p {
            p if p >= 0.7 => SelectionChance::High,
            p if p >= 0.3 => SelectionChance::Good,
            _ => SelectionChance::Low,
        }
    }
}

impl From<Decimal> for SelectionChance {
    fn from(p: Decimal) -> Self {
        match p {
            p if p >= Decimal::from_ratio(70u32, 100u32) => SelectionChance::High,
            p if p >= Decimal::from_ratio(30u32, 100u32) => SelectionChance::Good,
            _ => SelectionChance::Low,
        }
    }
}

impl fmt::Display for SelectionChance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionChance::High => write!(f, "High"),
            SelectionChance::Good => write!(f, "Good"),
            SelectionChance::Low => write!(f, "Low"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/InclusionProbabilityResponse.ts"
    )
)]
#[deprecated]
pub struct InclusionProbabilityResponse {
    pub in_active: SelectionChance,
    pub in_reserve: SelectionChance,
}

impl fmt::Display for InclusionProbabilityResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "in_active: {}, in_reserve: {}",
            self.in_active, self.in_reserve
        )
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, ToSchema, IntoParams)]
pub struct ComputeRewardEstParam {
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub performance: Option<Performance>,
    pub active_in_rewarded_set: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
    #[schema(value_type = Option<CoinSchema>)]
    #[param(value_type = Option<CoinSchema>)]
    pub interval_operating_cost: Option<Coin>,
    #[schema(value_type = Option<String>)]
    #[param(value_type = Option<String>)]
    pub profit_margin_percent: Option<Percent>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/RewardEstimationResponse.ts"
    )
)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RewardEstimationResponse {
    pub estimation: RewardEstimate,
    pub reward_params: RewardingParams,
    pub epoch: Interval,
    #[cfg_attr(feature = "generate-ts", ts(type = "number"))]
    pub as_at: i64,
}

impl MixNodeBondAnnotated {
    pub fn mix_node(&self) -> &MixNode {
        &self.mixnode_details.bond_information.mix_node
    }

    pub fn mix_id(&self) -> NodeId {
        self.mixnode_details.mix_id()
    }

    pub fn identity_key(&self) -> &str {
        self.mixnode_details.bond_information.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.mixnode_details.bond_information.owner()
    }

    pub fn version(&self) -> &str {
        &self.mixnode_details.bond_information.mix_node.version
    }

    pub fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        Ok(SkimmedNode {
            node_id: self.mix_id(),
            ed25519_identity_pubkey: self
                .identity_key()
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidEd25519Key)?,
            ip_addresses: self.ip_addresses.clone(),
            mix_port: self.mix_node().mix_port,
            x25519_sphinx_pubkey: self
                .mix_node()
                .sphinx_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidX25519Key)?,
            role,
            supported_roles: DeclaredRoles {
                mixnode: true,
                entry: false,
                exit_nr: false,
                exit_ipr: false,
            },
            entry: None,
            performance: self.node_performance.last_24h,
        })
    }

    pub fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        let skimmed_node = self.try_to_skimmed_node(role)?;
        Ok(SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: None, // legacy node won't ever support Noise
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayBondAnnotated {
    pub gateway_bond: LegacyGatewayBondWithId,

    #[serde(default)]
    pub self_described: Option<GatewayDescription>,

    // NOTE: the performance field is deprecated in favour of node_performance
    #[schema(value_type = String)]
    pub performance: Performance,
    pub node_performance: NodePerformance,
    pub blacklisted: bool,

    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}

impl GatewayBondAnnotated {
    pub fn version(&self) -> &str {
        &self.gateway_bond.gateway.version
    }

    pub fn identity(&self) -> &String {
        self.gateway_bond.bond.identity()
    }

    pub fn owner(&self) -> &Addr {
        self.gateway_bond.bond.owner()
    }

    pub fn try_to_skimmed_node(&self, role: NodeRole) -> Result<SkimmedNode, MalformedNodeBond> {
        Ok(SkimmedNode {
            node_id: self.gateway_bond.node_id,
            ip_addresses: self.ip_addresses.clone(),
            ed25519_identity_pubkey: self
                .gateway_bond
                .gateway
                .identity_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidEd25519Key)?,
            mix_port: self.gateway_bond.bond.gateway.mix_port,
            x25519_sphinx_pubkey: self
                .gateway_bond
                .gateway
                .sphinx_key
                .parse()
                .map_err(|_| MalformedNodeBond::InvalidX25519Key)?,
            role,
            supported_roles: DeclaredRoles {
                mixnode: false,
                entry: true,
                exit_nr: false,
                exit_ipr: false,
            },
            entry: Some(BasicEntryInformation {
                hostname: None,
                ws_port: self.gateway_bond.bond.gateway.clients_port,
                wss_port: None,
            }),
            performance: self.node_performance.last_24h,
        })
    }

    pub fn try_to_semi_skimmed_node(
        &self,
        role: NodeRole,
    ) -> Result<SemiSkimmedNode, MalformedNodeBond> {
        let skimmed_node = self.try_to_skimmed_node(role)?;
        Ok(SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: None, // legacy node won't ever support Noise
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct GatewayDescription {
    // for now only expose what we need. this struct will evolve in the future (or be incorporated into nym-node properly)
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
#[schema(title = "LegacyMixNodeDetailsWithLayer")]
pub struct LegacyMixNodeDetailsWithLayerSchema {
    /// Basic bond information of this mixnode, such as owner address, original pledge, etc.
    #[schema(example = "unimplemented schema")]
    pub bond_information: String,

    /// Details used for computation of rewarding related data.
    #[schema(example = "unimplemented schema")]
    pub rewarding_details: String,

    /// Adjustments to the mixnode that are ought to happen during future epoch transitions.
    #[schema(example = "unimplemented schema")]
    pub pending_changes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct MixNodeBondAnnotated {
    #[schema(value_type = LegacyMixNodeDetailsWithLayerSchema)]
    pub mixnode_details: LegacyMixNodeDetailsWithLayer,
    #[schema(value_type = String)]
    pub stake_saturation: StakeSaturation,
    #[schema(value_type = String)]
    pub uncapped_stake_saturation: StakeSaturation,
    // NOTE: the performance field is deprecated in favour of node_performance
    #[schema(value_type = String)]
    pub performance: Performance,
    pub node_performance: NodePerformance,
    #[schema(value_type = String)]
    pub estimated_operator_apy: Decimal,
    #[schema(value_type = String)]
    pub estimated_delegators_apy: Decimal,
    pub blacklisted: bool,

    // a rather temporary thing until we query self-described endpoints of mixnodes
    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}

#[derive(Debug, Error)]
pub enum MalformedNodeBond {
    #[error("the associated ed25519 identity key is malformed")]
    InvalidEd25519Key,

    #[error("the associated x25519 sphinx key is malformed")]
    InvalidX25519Key,
}
