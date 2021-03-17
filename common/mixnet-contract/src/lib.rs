use cosmwasm_std::{Coin, HumanAddr};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(target_arch = "wasm32")]
use schemars::JsonSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(JsonSchema))]
pub struct MixNode {
    pub(crate) host: String,
    pub(crate) layer: u64,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(JsonSchema))]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(JsonSchema))]
pub struct Gateway {
    pub(crate) mix_host: String,
    pub(crate) clients_host: String,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(JsonSchema))]
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
