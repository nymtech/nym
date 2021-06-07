use serde::{Deserialize, Serialize};

mod gateway;
mod mixnode;

pub use cosmwasm_std::{Coin, HumanAddr};
pub use gateway::{Gateway, GatewayBond, GatewayOwnershipResponse, PagedGatewayResponse};
pub use mixnode::{MixNode, MixNodeBond, MixOwnershipResponse, PagedResponse};

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct LayerDistribution {
    pub gateways: u64,
    pub layer1: u64,
    pub layer2: u64,
    pub layer3: u64,
    pub invalid: u64,
}
