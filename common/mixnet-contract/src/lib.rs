pub use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixNode {
    pub(crate) host: String,
    pub(crate) layer: u64,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key.
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

impl MixNode {
    pub fn new(
        host: String,
        layer: u64,
        location: String,
        sphinx_key: String,
        identity_key: String,
        version: String,
    ) -> Self {
        MixNode {
            host,
            layer,
            location,
            sphinx_key,
            identity_key,
            version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixNodeBond {
    pub(crate) amount: Vec<Coin>,
    pub(crate) owner: HumanAddr,
    pub(crate) mix_node: MixNode,
}

impl MixNodeBond {
    pub fn new(amount: Vec<Coin>, owner: HumanAddr, mix_node: MixNode) -> Self {
        MixNodeBond {
            amount,
            owner,
            mix_node,
        }
    }

    pub fn amount(&self) -> &[Coin] {
        &self.amount
    }

    pub fn owner(&self) -> &HumanAddr {
        &self.owner
    }

    pub fn mix_node(&self) -> &MixNode {
        &self.mix_node
    }
}

impl Display for MixNodeBond {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "amount: {:?}, owner: {}", self.amount, self.owner)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct Gateway {
    pub(crate) mix_host: String,
    pub(crate) clients_host: String,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

impl Gateway {
    pub fn new(
        mix_host: String,
        clients_host: String,
        location: String,
        sphinx_key: String,
        identity_key: String,
        version: String,
    ) -> Self {
        Gateway {
            mix_host,
            clients_host,
            location,
            sphinx_key,
            identity_key,
            version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct GatewayBond {
    pub(crate) amount: Vec<Coin>,
    pub(crate) owner: HumanAddr,
    pub(crate) gateway: Gateway,
}

impl GatewayBond {
    pub fn new(amount: Vec<Coin>, owner: HumanAddr, gateway: Gateway) -> Self {
        GatewayBond {
            amount,
            owner,
            gateway,
        }
    }

    pub fn amount(&self) -> &[Coin] {
        &self.amount
    }

    pub fn owner(&self) -> &HumanAddr {
        &self.owner
    }

    pub fn gateway(&self) -> &Gateway {
        &self.gateway
    }
}

impl Display for GatewayBond {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.amount.len() != 1 {
            write!(f, "amount: {:?}, owner: {}", self.amount, self.owner)
        } else {
            write!(
                f,
                "amount: {} {}, owner: {}",
                self.amount[0].amount, self.amount[0].denom, self.owner
            )
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedResponse {
    pub nodes: Vec<MixNodeBond>,
    pub per_page: usize,
    pub start_next_after: Option<HumanAddr>,
}

impl PagedResponse {
    pub fn new(
        nodes: Vec<MixNodeBond>,
        per_page: usize,
        start_next_after: Option<HumanAddr>,
    ) -> Self {
        PagedResponse {
            nodes,
            per_page,
            start_next_after,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedGatewayResponse {
    pub nodes: Vec<GatewayBond>,
    pub per_page: usize,
    pub start_next_after: Option<HumanAddr>,
}

impl PagedGatewayResponse {
    pub fn new(
        nodes: Vec<GatewayBond>,
        per_page: usize,
        start_next_after: Option<HumanAddr>,
    ) -> Self {
        PagedGatewayResponse {
            nodes,
            per_page,
            start_next_after,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixOwnershipResponse {
    pub address: HumanAddr,
    pub has_node: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct GatewayOwnershipResponse {
    pub address: HumanAddr,
    pub has_gateway: bool,
}
