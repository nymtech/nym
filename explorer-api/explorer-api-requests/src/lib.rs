use nym_api_requests::models::{DescribedNodeType, NodePerformance, NymNodeData};
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{
    Addr, Coin, Delegation, Gateway, LegacyMixLayer, MixNode, NodeId, NodeRewarding, NymNodeBond,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrettyDetailedMixNodeBond {
    pub mix_id: NodeId,
    pub location: Option<Location>,
    pub status: MixnodeStatus,
    pub pledge_amount: Coin,
    pub total_delegation: Coin,
    pub owner: Addr,
    pub layer: LegacyMixLayer,
    pub mix_node: MixNode,
    pub stake_saturation: f32,
    pub uncapped_saturation: f32,
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
    pub estimated_operator_apy: f64,
    pub estimated_delegators_apy: f64,
    pub operating_cost: Coin,
    pub profit_margin_percent: Percent,
    pub family_id: Option<u16>,
    pub blacklisted: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub struct Location {
    pub two_letter_iso_country_code: String,
    pub three_letter_iso_country_code: String,
    pub country_name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PrettyDetailedGatewayBond {
    pub pledge_amount: Coin,
    pub owner: Addr,
    pub block_height: u64,
    pub gateway: Gateway,
    pub proxy: Option<Addr>,
    pub location: Option<Location>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NymNodeWithDescriptionAndLocation {
    pub node_id: NodeId,
    pub contract_node_type: Option<DescribedNodeType>,
    pub description: Option<NymNodeData>,
    pub bond_information: NymNodeBond,
    pub rewarding_details: NodeRewarding,
    pub location: Option<Location>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NymNodeWithDescriptionAndLocationAndDelegations {
    pub node_id: NodeId,
    pub contract_node_type: Option<DescribedNodeType>,
    pub description: Option<NymNodeData>,
    pub bond_information: NymNodeBond,
    pub rewarding_details: NodeRewarding,
    pub location: Option<Location>,
    pub delegations: Option<Vec<Delegation>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NymVestingAccount {
    pub locked: Coin,
    pub vested: Coin,
    pub vesting: Coin,
    pub spendable: Coin,
}
