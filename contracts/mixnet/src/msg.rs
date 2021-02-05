// use cosmwasm_std::HumanAddr;
use crate::types::MixNode;
use crate::types::MixNodeBond;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// use validator_client::models::mixnode::RegisteredMix;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RegisterMixnode { mix_node: MixNode },
    UnRegisterMixnode {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTopology {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Topology {
    pub mix_node_bonds: Vec<MixNodeBond>,
}
