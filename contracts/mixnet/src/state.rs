use cosmwasm_std::{Coin, Order, StdError, StdResult};
use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, prefixed_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    ReadonlySingleton, Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Contract-level stuff

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub mix_node_bonds: Vec<MixNodeBond>, // TODO: whack this, we need to use a range instead
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

pub fn mixnodes_all(storage: &dyn Storage) -> StdResult<Vec<String>> {
    prefixed_read(storage, PREFIX_MIXNODES)
        .range(None, None, Order::Ascending)
        .map(|(k, _)| {
            // from_binary(v.into())
            String::from_utf8(k).map_err(|_| StdError::invalid_utf8("parsing mix node bond key"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn empty_on_init() {
        let storage = MockStorage::new();
        let all_nodes = mixnodes_all(&storage).unwrap();
        assert_eq!(0, all_nodes.len());
    }
}
