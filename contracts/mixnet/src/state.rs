use cosmwasm_std::{Coin, Order, StdResult};
use cosmwasm_std::{HumanAddr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
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

pub fn mixnodes_all(storage: &dyn Storage) -> Vec<MixNodeBond> {
    let bucket = bucket_read::<MixNodeBond>(storage, PREFIX_MIXNODES);
    let query_result: StdResult<Vec<(Vec<u8>, MixNodeBond)>> =
        bucket.range(None, None, Order::Ascending).collect();
    let node_tuples = query_result.unwrap();
    let nodes = node_tuples.into_iter().map(|item| item.1).collect();
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, testing::MockStorage};

    fn mixnode_bond_fixture() -> MixNodeBond {
        let mix_node = MixNode {
            host: "1.1.1.1".to_string(),
            layer: 1,
            location: "London".to_string(),
            sphinx_key: "1234".to_string(),
            version: "0.10.0".to_string(),
        };
        MixNodeBond {
            amount: coins(50, "unym"),
            owner: HumanAddr::from("foo"),
            mix_node,
        }
    }

    #[test]
    fn mixnodes_empty_on_init() {
        let storage = MockStorage::new();
        let all_nodes = mixnodes_all(&storage);
        assert_eq!(0, all_nodes.len());
    }

    #[test]
    fn mixnodes_range_retrieval_works() {
        let mut storage = MockStorage::new();
        let bond1 = mixnode_bond_fixture();
        let bond2 = mixnode_bond_fixture();
        mixnodes(&mut storage).save(b"bond1", &bond1).unwrap();
        mixnodes(&mut storage).save(b"bond2", &bond2).unwrap();
        let all_nodes = mixnodes_all(&storage);
        assert_eq!(2, all_nodes.len());
    }

    #[test]
    fn mixnodes_retrieval_works_with_large_numbers_of_nodes() {
        let mut storage = MockStorage::new();
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = mixnode_bond_fixture();
            mixnodes(&mut storage).save(key.as_bytes(), &node).unwrap();
        }
        let all_nodes = mixnodes_all(&storage);
        assert_eq!(10000, all_nodes.len());
    }
}
