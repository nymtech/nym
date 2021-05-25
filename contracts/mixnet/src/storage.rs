use crate::state::{State, StateParams};
use cosmwasm_std::{Decimal, HumanAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use mixnet_contract::{GatewayBond, MixNodeBond};

// Contract-level stuff
const CONFIG_KEY: &[u8] = b"config";

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub(crate) fn read_state_params(storage: &dyn Storage) -> StateParams {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    config_read(storage).load().unwrap().params
}

pub(crate) fn read_mixnode_epoch_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .mixnode_epoch_bond_reward
}

pub(crate) fn read_gateway_epoch_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .gateway_epoch_bond_reward
}

// Mixnode-related stuff
const PREFIX_MIXNODES: &[u8] = b"mixnodes";

pub fn mixnodes(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<MixNodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

const PREFIX_MIXNODES_OWNERS: &[u8] = b"mix-owners";

pub fn mixnodes_owners(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn mixnodes_owners_read(storage: &dyn Storage) -> ReadonlyBucket<HumanAddr> {
    bucket_read(storage, PREFIX_MIXNODES_OWNERS)
}

// helpers
pub(crate) fn increase_mixnode_bond(
    storage: &mut dyn Storage,
    owner: &[u8],
    scaled_reward_rate: Decimal,
) -> StdResult<()> {
    let mut bucket = mixnodes(storage);
    let mut node = bucket.load(owner)?;
    if node.amount.len() != 1 {
        return Err(StdError::generic_err(
            "mixnode seems to have been bonded with multiple coin types",
        ));
    }

    let reward = node.amount[0].amount * scaled_reward_rate;
    node.amount[0].amount += reward;
    bucket.save(owner, &node)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_mixnode_bond(
    storage: &dyn Storage,
    owner: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = mixnodes_read(storage);
    let node = bucket.load(owner)?;
    if node.amount.len() != 1 {
        return Err(StdError::generic_err(
            "mixnode seems to have been bonded with multiple coin types",
        ));
    }
    Ok(node.amount[0].amount)
}

// Gateway-related stuff

const PREFIX_GATEWAYS: &[u8] = b"gateways";

pub fn gateways(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
    bucket(storage, PREFIX_GATEWAYS)
}

pub fn gateways_read(storage: &dyn Storage) -> ReadonlyBucket<GatewayBond> {
    bucket_read(storage, PREFIX_GATEWAYS)
}

const PREFIX_GATEWAYS_OWNERS: &[u8] = b"gateway-owners";

pub fn gateways_owners(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_GATEWAYS_OWNERS)
}

pub fn gateways_owners_read(storage: &dyn Storage) -> ReadonlyBucket<HumanAddr> {
    bucket_read(storage, PREFIX_GATEWAYS_OWNERS)
}

// helpers
pub(crate) fn increase_gateway_bond(
    storage: &mut dyn Storage,
    owner: &[u8],
    scaled_reward_rate: Decimal,
) -> StdResult<()> {
    let mut bucket = gateways(storage);
    let mut node = bucket.load(owner)?;
    if node.amount.len() != 1 {
        return Err(StdError::generic_err(
            "gateway seems to have been bonded with multiple coin types",
        ));
    }
    let reward = node.amount[0].amount * scaled_reward_rate;
    node.amount[0].amount += reward;
    bucket.save(owner, &node)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_gateway_bond(
    storage: &dyn Storage,
    owner: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = gateways_read(storage);
    let node = bucket.load(owner)?;
    if node.amount.len() != 1 {
        return Err(StdError::generic_err(
            "gateway seems to have been bonded with multiple coin types",
        ));
    }
    Ok(node.amount[0].amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::DENOM;
    use crate::support::tests::helpers::{
        gateway_bond_fixture, gateway_fixture, mix_node_fixture, mixnode_bond_fixture,
    };
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{coins, Uint128};

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

    #[test]
    fn increasing_mixnode_bond() {
        let mut storage = MockStorage::new();
        let node_owner = b"owner";
        // 0.001
        let reward = Decimal::from_ratio(1u128, 1000u128);

        // produces an error if target mixnode doesn't exist
        let res = increase_mixnode_bond(&mut storage, node_owner, reward);
        assert!(res.is_err());

        // increases the reward appropriately if node exists
        let mixnode_bond = MixNodeBond {
            amount: coins(1000, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            mix_node: mix_node_fixture(),
        };

        mixnodes(&mut storage)
            .save(node_owner, &mixnode_bond)
            .unwrap();

        increase_mixnode_bond(&mut storage, node_owner, reward).unwrap();
        let new_bond = read_mixnode_bond(&storage, node_owner).unwrap();
        assert_eq!(Uint128(1001), new_bond);
    }

    #[test]
    fn reading_mixnode_bond() {
        let mut storage = MockStorage::new();
        let node_owner = b"owner";

        // produces an error if target mixnode doesn't exist
        let res = read_mixnode_bond(&storage, node_owner);
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let mixnode_bond = MixNodeBond {
            amount: coins(bond_value, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            mix_node: mix_node_fixture(),
        };

        mixnodes(&mut storage)
            .save(node_owner, &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_mixnode_bond(&storage, node_owner).unwrap()
        );
    }

    #[test]
    fn increasing_gateway_bond() {
        let mut storage = MockStorage::new();
        let node_owner = b"owner";
        // 0.001
        let reward = Decimal::from_ratio(1u128, 1000u128);

        // produces an error if target gateway doesn't exist
        let res = increase_gateway_bond(&mut storage, node_owner, reward);
        assert!(res.is_err());

        // increases the reward appropriately if node exists
        let gateway_bond = GatewayBond {
            amount: coins(1000, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            gateway: gateway_fixture(),
        };

        gateways(&mut storage)
            .save(node_owner, &gateway_bond)
            .unwrap();

        increase_gateway_bond(&mut storage, node_owner, reward).unwrap();
        let new_bond = read_gateway_bond(&storage, node_owner).unwrap();
        assert_eq!(Uint128(1001), new_bond);
    }

    #[test]
    fn reading_gateway_bond() {
        let mut storage = MockStorage::new();
        let node_owner = b"owner";

        // produces an error if target mixnode doesn't exist
        let res = read_gateway_bond(&storage, node_owner);
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let gateway_bond = GatewayBond {
            amount: coins(1000, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            gateway: gateway_fixture(),
        };

        gateways(&mut storage)
            .save(node_owner, &gateway_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_gateway_bond(&storage, node_owner).unwrap()
        );
    }
}
