use crate::queries;
use crate::state::{State, StateParams};
use cosmwasm_std::{Decimal, HumanAddr, Order, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use mixnet_contract::{GatewayBond, LayerDistribution, MixNodeBond};

// storage prefixes
// all of them must be unique.
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// the legacy prefixes can't be changed without redeploying contract or doing fancy migration
// for time being let's leave them as they are, but when we're going to make incompatible
// contract changes, let's shorten all of them

// singletons
const CONFIG_KEY: &[u8] = b"config";
const LAYER_DISTRIBUTION_KEY: &[u8] = b"layers";

// buckets
const PREFIX_MIXNODES: &[u8] = b"mixnodes";
const PREFIX_MIXNODES_OWNERS: &[u8] = b"mix-owners";
const PREFIX_GATEWAYS: &[u8] = b"gateways";
const PREFIX_GATEWAYS_OWNERS: &[u8] = b"gateway-owners";

const PREFIX_MIX_DELEGATION: &[u8] = b"md";
const PREFIX_GATEWAY_DELEGATION: &[u8] = b"gd";

// Contract-level stuff

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

pub fn layer_distribution(storage: &mut dyn Storage) -> Singleton<LayerDistribution> {
    singleton(storage, LAYER_DISTRIBUTION_KEY)
}

pub fn layer_distribution_read(storage: &dyn Storage) -> ReadonlySingleton<LayerDistribution> {
    singleton_read(storage, LAYER_DISTRIBUTION_KEY)
}

pub(crate) fn read_layer_distribution(storage: &dyn Storage) -> LayerDistribution {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    layer_distribution_read(storage).load().unwrap()
}

pub enum Layer {
    Gateway,
    One,
    Two,
    Three,
    Invalid,
}

impl From<u64> for Layer {
    fn from(val: u64) -> Self {
        match val {
            n if n == 1 => Layer::One,
            n if n == 2 => Layer::Two,
            n if n == 3 => Layer::Three,
            _ => Layer::Invalid,
        }
    }
}

pub fn increment_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    match layer {
        Layer::Gateway => distribution.gateways += 1,
        Layer::One => distribution.layer1 += 1,
        Layer::Two => distribution.layer2 += 1,
        Layer::Three => distribution.layer3 += 1,
        Layer::Invalid => distribution.invalid += 1,
    }
    layer_distribution(storage).save(&distribution)
}

pub fn decrement_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    // It can't possibly go below zero, if it does, it means there's a serious error in the contract logic
    match layer {
        Layer::Gateway => {
            distribution.gateways = distribution
                .gateways
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::One => {
            distribution.layer1 = distribution
                .layer1
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Two => {
            distribution.layer2 = distribution
                .layer2
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Three => {
            distribution.layer3 = distribution
                .layer3
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Invalid => {
            distribution.invalid = distribution
                .invalid
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
    };
    layer_distribution(storage).save(&distribution)
}

// Mixnode-related stuff

pub fn mixnodes(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<MixNodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_owners(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn mixnodes_owners_read(storage: &dyn Storage) -> ReadonlyBucket<HumanAddr> {
    bucket_read(storage, PREFIX_MIXNODES_OWNERS)
}

// helpers
pub(crate) fn increase_mixnode_bond(
    storage: &mut dyn Storage,
    mut bond: MixNodeBond,
    scaled_reward_rate: Decimal,
) -> StdResult<()> {
    if bond.amount.len() != 1 {
        return Err(StdError::generic_err(
            "mixnode seems to have been bonded with multiple coin types",
        ));
    }

    let reward = bond.amount[0].amount * scaled_reward_rate;
    bond.amount[0].amount += reward;
    mixnodes(storage).save(bond.owner.as_bytes(), &bond)
}

pub(crate) fn increase_mix_delegated_stakes(
    storage: &mut dyn Storage,
    node_owner: &HumanAddr,
    scaled_reward_rate: Decimal,
) -> StdResult<()> {
    let chunk_size = queries::DELEGATION_PAGE_MAX_LIMIT as usize;

    let mut chunk_start: Option<Vec<_>> = None;
    loop {
        // get `chunk_size` of delegations
        let delegations_chunk = mix_delegations_read(storage, node_owner)
            .range(chunk_start.as_deref(), None, Order::Ascending)
            .take(chunk_size)
            .collect::<StdResult<Vec<_>>>()?;

        if delegations_chunk.is_empty() {
            break;
        }

        // append 0 byte to the last value to start with whatever is the next suceeding key
        chunk_start = Some(
            delegations_chunk
                .last()
                .unwrap()
                .0
                .iter()
                .cloned()
                .chain(std::iter::once(0u8))
                .collect(),
        );

        // and for each of them increase the stake proportionally to the reward
        for (delegator_address, amount) in delegations_chunk.into_iter() {
            let reward = amount * scaled_reward_rate;
            let new_amount = amount + reward;
            mix_delegations(storage, node_owner).save(&delegator_address, &new_amount)?;
        }
    }

    Ok(())
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

pub fn gateways(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
    bucket(storage, PREFIX_GATEWAYS)
}

pub fn gateways_read(storage: &dyn Storage) -> ReadonlyBucket<GatewayBond> {
    bucket_read(storage, PREFIX_GATEWAYS)
}

pub fn gateways_owners(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_GATEWAYS_OWNERS)
}

pub fn gateways_owners_read(storage: &dyn Storage) -> ReadonlyBucket<HumanAddr> {
    bucket_read(storage, PREFIX_GATEWAYS_OWNERS)
}

// helpers
pub(crate) fn increase_gateway_bond(
    storage: &mut dyn Storage,
    mut bond: GatewayBond,
    scaled_reward_rate: Decimal,
) -> StdResult<()> {
    if bond.amount.len() != 1 {
        return Err(StdError::generic_err(
            "gateway seems to have been bonded with multiple coin types",
        ));
    }
    let reward = bond.amount[0].amount * scaled_reward_rate;
    bond.amount[0].amount += reward;
    gateways(storage).save(bond.owner.as_bytes(), &bond)
}

// delegation related

pub fn mix_delegations<'a>(
    storage: &'a mut dyn Storage,
    node_address: &'a HumanAddr,
) -> Bucket<'a, Uint128> {
    Bucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, node_address.as_bytes()])
}

pub fn mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    node_address: &'a HumanAddr,
) -> ReadonlyBucket<'a, Uint128> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, node_address.as_bytes()])
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

        // increases the reward appropriately
        let mixnode_bond = MixNodeBond {
            amount: coins(1000, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            mix_node: mix_node_fixture(),
        };

        mixnodes(&mut storage)
            .save(node_owner, &mixnode_bond)
            .unwrap();

        increase_mixnode_bond(&mut storage, mixnode_bond, reward).unwrap();
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

        // increases the reward appropriately
        let gateway_bond = GatewayBond {
            amount: coins(1000, DENOM),
            owner: std::str::from_utf8(node_owner).unwrap().into(),
            gateway: gateway_fixture(),
        };

        gateways(&mut storage)
            .save(node_owner, &gateway_bond)
            .unwrap();

        increase_gateway_bond(&mut storage, gateway_bond, reward).unwrap();
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

    #[cfg(test)]
    mod increasing_mix_delegated_stakes {
        use super::*;
        use crate::queries::query_mixnode_delegations_paged;
        use cosmwasm_std::testing::mock_dependencies;

        #[test]
        fn when_there_are_no_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_owner = HumanAddr::from("owner");
            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            increase_mix_delegated_stakes(&mut deps.storage, &node_owner, reward).unwrap();

            // there are no 'new' delegations magically added
            assert!(
                query_mixnode_delegations_paged(deps.as_ref(), node_owner, None, None)
                    .unwrap()
                    .delegations
                    .is_empty()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation() {
            let mut deps = mock_dependencies(&[]);
            let node_owner = HumanAddr::from("owner");
            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = HumanAddr::from("bob");
            mix_delegations(&mut deps.storage, &node_owner)
                .save(delegator_address.as_bytes(), &Uint128(1000))
                .unwrap();

            increase_mix_delegated_stakes(&mut deps.storage, &node_owner, reward).unwrap();
            assert_eq!(
                Uint128(1001),
                mix_delegations_read(&mut deps.storage, &node_owner)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_are_multiple_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_owner = HumanAddr::from("owner");
            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..100 {
                let delegator_address = HumanAddr::from(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_owner)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            increase_mix_delegated_stakes(&mut deps.storage, &node_owner, reward).unwrap();

            for i in 0..100 {
                let delegator_address = HumanAddr::from(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    mix_delegations_read(&mut deps.storage, &node_owner)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }

        #[test]
        fn when_there_are_more_delegations_than_page_size() {
            let mut deps = mock_dependencies(&[]);
            let node_owner = HumanAddr::from("owner");
            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = HumanAddr::from(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_owner)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            increase_mix_delegated_stakes(&mut deps.storage, &node_owner, reward).unwrap();

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = HumanAddr::from(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    mix_delegations_read(&mut deps.storage, &node_owner)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }
}
