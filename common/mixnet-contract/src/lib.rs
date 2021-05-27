mod gateway;
mod mixnode;

pub use cosmwasm_std::{Coin, HumanAddr};
pub use gateway::{Gateway, GatewayBond, GatewayOwnershipResponse, PagedGatewayResponse};
pub use mixnode::{MixNode, MixNodeBond, MixOwnershipResponse, PagedResponse};

pub struct LayerDistribution {
    pub layer1: u32,
    pub layer2: u32,
    pub layer3: u32,
    pub gateways: u32,
}
