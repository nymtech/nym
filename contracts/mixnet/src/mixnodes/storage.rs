use crate::error::ContractError;
use crate::rewards::transactions::MINIMUM_BLOCK_AGE_FOR_REWARDING;
use cosmwasm_std::Addr;
use cosmwasm_std::Decimal;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cosmwasm_std::Storage;
use cosmwasm_std::Uint128;
use cosmwasm_storage::bucket;
use cosmwasm_storage::bucket_read;
use cosmwasm_storage::Bucket;
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::mixnode::NodeRewardParams;
use mixnet_contract::IdentityKey;
use mixnet_contract::IdentityKeyRef;
use mixnet_contract::MixNodeBond;
use mixnet_contract::RawDelegationData;
use serde::de::DeserializeOwned;
use serde::Serialize;

// storage prefixes
const PREFIX_MIXNODES: &[u8] = b"mn";
const PREFIX_MIXNODES_OWNERS: &[u8] = b"mo";
const PREFIX_MIX_DELEGATION: &[u8] = b"md";
const PREFIX_REVERSE_MIX_DELEGATION: &[u8] = b"dm";
pub const PREFIX_REWARDED_MIXNODES: &[u8] = b"rm";

// paged retrieval limits for all queries and transactions
// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 750;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 500;
pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 100;

pub fn mixnodes(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<MixNodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

// owner address -> node identity
pub fn mixnodes_owners(storage: &mut dyn Storage) -> Bucket<IdentityKey> {
    bucket(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn mixnodes_owners_read(storage: &dyn Storage) -> ReadonlyBucket<IdentityKey> {
    bucket_read(storage, PREFIX_MIXNODES_OWNERS)
}

// we want to treat this bucket as a set so we don't really care about what type of data is being stored.
// I went with u8 as after serialization it takes only a single byte of space, while if a `()` was used,
// it would have taken 4 bytes (representation of 'null')
pub fn rewarded_mixnodes(storage: &mut dyn Storage, rewarding_interval_nonce: u32) -> Bucket<u8> {
    Bucket::multilevel(
        storage,
        &[
            rewarding_interval_nonce.to_be_bytes().as_ref(),
            PREFIX_REWARDED_MIXNODES,
        ],
    )
}

// we want to treat this bucket as a set so we don't really care about what type of data is being stored.
// I went with u8 as after serialization it takes only a single byte of space, while if a `()` was used,
// it would have taken 4 bytes (representation of 'null')
pub fn rewarded_mixnodes_read(
    storage: &dyn Storage,
    rewarding_interval_nonce: u32,
) -> ReadonlyBucket<u8> {
    ReadonlyBucket::multilevel(
        storage,
        &[
            rewarding_interval_nonce.to_be_bytes().as_ref(),
            PREFIX_REWARDED_MIXNODES,
        ],
    )
}

// helpers
pub(crate) fn increase_mix_delegated_stakes(
    storage: &mut dyn Storage,
    mix_identity: IdentityKeyRef,
    scaled_reward_rate: Decimal,
    reward_blockstamp: u64,
) -> StdResult<Uint128> {
    let chunk_size = DELEGATION_PAGE_MAX_LIMIT as usize;

    let mut total_rewarded = Uint128::zero();
    let mut chunk_start: Option<Vec<_>> = None;
    loop {
        // get `chunk_size` of delegations
        let delegations_chunk = mix_delegations_read(storage, mix_identity)
            .range(chunk_start.as_deref(), None, Order::Ascending)
            .take(chunk_size)
            .collect::<StdResult<Vec<_>>>()?;

        if delegations_chunk.is_empty() {
            break;
        }

        // append 0 byte to the last value to start with whatever is the next succeeding key
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
        // if at least `MINIMUM_BLOCK_AGE_FOR_REWARDING` blocks have been created
        // since they delegated
        for (delegator_address, mut delegation) in delegations_chunk.into_iter() {
            if delegation.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING <= reward_blockstamp {
                let reward = delegation.amount * scaled_reward_rate;
                delegation.amount += reward;
                total_rewarded += reward;
                mix_delegations(storage, mix_identity).save(&delegator_address, &delegation)?;
            }
        }
    }

    Ok(total_rewarded)
}

pub(crate) fn increase_mix_delegated_stakes_v2(
    storage: &mut dyn Storage,
    bond: &MixNodeBond,
    params: &NodeRewardParams,
) -> Result<Uint128, ContractError> {
    let chunk_size = DELEGATION_PAGE_MAX_LIMIT as usize;

    let mut total_rewarded = Uint128::zero();
    let mut chunk_start: Option<Vec<_>> = None;
    loop {
        // get `chunk_size` of delegations
        let delegations_chunk = mix_delegations_read(storage, bond.identity())
            .range(chunk_start.as_deref(), None, Order::Ascending)
            .take(chunk_size)
            .collect::<StdResult<Vec<_>>>()?;

        if delegations_chunk.is_empty() {
            break;
        }

        // append 0 byte to the last value to start with whatever is the next succeeding key
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
        // if at least `MINIMUM_BLOCK_AGE_FOR_REWARDING` blocks have been created
        // since they delegated
        for (delegator_address, mut delegation) in delegations_chunk.into_iter() {
            if delegation.block_height + MINIMUM_BLOCK_AGE_FOR_REWARDING
                <= params.reward_blockstamp()
            {
                let reward = bond.reward_delegation(delegation.amount, params);
                delegation.amount += Uint128(reward);
                total_rewarded += Uint128(reward);
                mix_delegations(storage, bond.identity()).save(&delegator_address, &delegation)?;
            }
        }
    }

    Ok(total_rewarded)
}
// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_mixnode_bond(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = mixnodes_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.bond_amount.amount)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_mixnode_delegation(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = mixnodes_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.total_delegation.amount)
}

// delegation related
pub fn all_mix_delegations_read<T>(storage: &dyn Storage) -> ReadonlyBucket<T>
where
    T: Serialize + DeserializeOwned,
{
    bucket_read(storage, PREFIX_MIX_DELEGATION)
}

pub fn mix_delegations<'a>(
    storage: &'a mut dyn Storage,
    mix_identity: IdentityKeyRef,
) -> Bucket<'a, RawDelegationData> {
    Bucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

pub fn mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    mix_identity: IdentityKeyRef,
) -> ReadonlyBucket<'a, RawDelegationData> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

pub fn reverse_mix_delegations<'a>(storage: &'a mut dyn Storage, owner: &Addr) -> Bucket<'a, ()> {
    Bucket::multilevel(storage, &[PREFIX_REVERSE_MIX_DELEGATION, owner.as_bytes()])
}

pub fn reverse_mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    owner: &Addr,
) -> ReadonlyBucket<'a, ()> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REVERSE_MIX_DELEGATION, owner.as_bytes()])
}

#[cfg(test)]
mod tests {
    use super::super::storage;
    use super::*;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, MockStorage};
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::IdentityKey;
    use mixnet_contract::Layer;
    use mixnet_contract::MixNode;
    use mixnet_contract::MixNodeBond;
    use mixnet_contract::RawDelegationData;

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = test_helpers::mixnode_bond_fixture();
        let bond2 = test_helpers::mixnode_bond_fixture();
        storage::mixnodes(&mut storage)
            .save(b"bond1", &bond1)
            .unwrap();
        storage::mixnodes(&mut storage)
            .save(b"bond2", &bond2)
            .unwrap();

        let res1 = storage::mixnodes_read(&storage).load(b"bond1").unwrap();
        let res2 = storage::mixnodes_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_mixnode_bond() {
        let mut storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target mixnode doesn't exist
        let res = storage::read_mixnode_bond(&storage, node_owner.as_bytes());
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let mixnode_bond = MixNodeBond {
            bond_amount: coin(bond_value, DENOM),
            total_delegation: coin(0, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: 12_345,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..test_helpers::mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        storage::mixnodes(&mut storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            storage::read_mixnode_bond(&storage, node_identity.as_bytes()).unwrap()
        );
    }

    #[test]
    fn all_mixnode_delegations_read_retrieval() {
        let mut deps = mock_dependencies(&[]);
        let node_identity1: IdentityKey = "foo1".into();
        let delegation_owner1 = Addr::unchecked("bar1");
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner2 = Addr::unchecked("bar2");
        let raw_delegation1 = RawDelegationData::new(1u128.into(), 1000);
        let raw_delegation2 = RawDelegationData::new(2u128.into(), 2000);

        storage::mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner1.as_bytes(), &raw_delegation1)
            .unwrap();
        storage::mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner2.as_bytes(), &raw_delegation2)
            .unwrap();

        let res1 = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*test_helpers::identity_and_owner_to_bytes(
                &node_identity1,
                &delegation_owner1,
            ))
            .unwrap();
        let res2 = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*test_helpers::identity_and_owner_to_bytes(
                &node_identity2,
                &delegation_owner2,
            ))
            .unwrap();
        assert_eq!(raw_delegation1, res1);
        assert_eq!(raw_delegation2, res2);
    }

    #[cfg(test)]
    mod increasing_mix_delegated_stakes {
        use super::*;
        use crate::mixnodes::bonding_queries::query_mixnode_delegations_paged;
        use crate::rewards::transactions::MINIMUM_BLOCK_AGE_FOR_REWARDING;
        use cosmwasm_std::testing::mock_dependencies;
        use cosmwasm_std::Decimal;

        #[test]
        fn when_there_are_no_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                42,
            )
            .unwrap();

            // there was no increase
            assert!(total_increase.is_zero());

            // there are no 'new' delegations magically added
            assert!(
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, None)
                    .unwrap()
                    .delegations
                    .is_empty()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            storage::mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(Uint128(1), total_increase);

            // amount is incremented, block height remains the same
            assert_eq!(
                RawDelegationData::new(1001u128.into(), 42),
                storage::mix_delegations_read(&deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation_depending_on_blockstamp() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            storage::mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + MINIMUM_BLOCK_AGE_FOR_REWARDING - 1,
            )
            .unwrap();

            // there was no increase
            assert!(total_increase.is_zero());

            // amount is not incremented
            assert_eq!(
                RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                storage::mix_delegations_read(&deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            );

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            // there is an increase now, that the lock period has passed
            assert_eq!(Uint128(1), total_increase);

            // amount is incremented
            assert_eq!(
                RawDelegationData::new(1001u128.into(), delegation_blockstamp),
                storage::mix_delegations_read(&deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_are_multiple_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                storage::mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(Uint128(100), total_increase);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    test_helpers::raw_delegation_fixture(1001),
                    storage::mix_delegations_read(&deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }

        #[test]
        fn when_there_are_more_delegations_than_page_size() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                storage::mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(
                Uint128(DELEGATION_PAGE_MAX_LIMIT as u128 * 10),
                total_increase
            );

            for i in 0..DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    test_helpers::raw_delegation_fixture(1001),
                    storage::mix_delegations_read(&deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }

    #[cfg(test)]
    mod reverse_mix_delegations {
        use super::*;
        use crate::support::tests::test_helpers;

        #[test]
        fn reverse_mix_delegation_exists() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");

            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save(node_identity.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner
            )
            .may_load(node_identity.as_bytes())
            .unwrap()
            .is_some(),);
        }

        #[test]
        fn reverse_mix_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = test_helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());

            // add delegation for a different node
            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner1)
                .save(node_identity2.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());

            // add delegation from a different owner
            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner2)
                .save(node_identity1.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());
        }
    }
}
