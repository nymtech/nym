use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr, // only the owner account can update state
    pub network_monitor_address: HumanAddr,
    pub params: StateParams,

    // helper values to avoid having to recalculate them on every single payment operation
    pub mixnode_epoch_bond_reward: Decimal, // reward per epoch expressed as a decimal like 0.05
    pub gateway_epoch_bond_reward: Decimal, // reward per epoch expressed as a decimal like 0.05
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateParams {
    pub epoch_length: u32, // length of an epoch, expressed in hours

    pub minimum_mixnode_bond: Uint128, // minimum amount a mixnode must bond to get into the system
    pub minimum_gateway_bond: Uint128, // minimum amount a gateway must bond to get into the system
    pub mixnode_bond_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub gateway_bond_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub mixnode_active_set_size: u32,
}
