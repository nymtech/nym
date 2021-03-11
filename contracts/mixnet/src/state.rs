use cosmwasm_std::Coin;
use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

// Contract-level stuff

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

// Mixnode-related stuff

pub const PREFIX_MIXNODES: &[u8] = b"mixnodes";
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixNodeBond {
    pub(crate) amount: Vec<Coin>,
    pub(crate) owner: HumanAddr,
    pub(crate) mix_node: MixNode,
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
pub struct MixNode {
    pub(crate) host: String,
    pub(crate) layer: u64,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    pub(crate) version: String,
}

pub fn mixnodes(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<MixNodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

// Gateway-related stuff

pub const PREFIX_GATEWAYS: &[u8] = b"gateways";

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
pub struct Gateway {
    pub(crate) mix_host: String,
    pub(crate) clients_host: String,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

pub fn gateways(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
    bucket(storage, PREFIX_GATEWAYS)
}

pub fn gateways_read(storage: &dyn Storage) -> ReadonlyBucket<GatewayBond> {
    bucket_read(storage, PREFIX_GATEWAYS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::{gateway_bond_fixture, mixnode_bond_fixture};
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = mixnode_bond_fixture();
        let bond2 = mixnode_bond_fixture();
        mixnodes(&mut storage).save(b"bond1", &bond1).unwrap();
        mixnodes(&mut storage).save(b"bond2", &bond2).unwrap();

        let res1 = mixnodes_read(&storage).load(b"bond1").unwrap();
        let res2 = mixnodes_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn gateway_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = gateway_bond_fixture();
        let bond2 = gateway_bond_fixture();
        gateways(&mut storage).save(b"bond1", &bond1).unwrap();
        gateways(&mut storage).save(b"bond2", &bond2).unwrap();

        let res1 = gateways_read(&storage).load(b"bond1").unwrap();
        let res2 = gateways_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }
}
