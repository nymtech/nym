use serde::{Deserialize, Serialize};

mod delegation;
mod gateway;
mod mixnode;

pub use cosmwasm_std::{Coin, HumanAddr};
pub use delegation::{Delegation, PagedGatewayDelegationsResponse, PagedMixDelegationsResponse};
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

// type aliases for better reasoning about available data
pub type IdentityKey = String;
pub type SphinxKey = String;
