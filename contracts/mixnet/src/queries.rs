// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{
    circulating_supply, config_read, read_layer_distribution, reward_pool_value,
};

use cosmwasm_std::{Deps, Uint128};
use mixnet_contract::{LayerDistribution, RewardingIntervalResponse};

pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 100;
pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 750;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 500;

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = config_read(deps.storage).load().unwrap();
    RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    }
}

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    read_layer_distribution(deps.storage)
}

pub(crate) fn query_reward_pool(deps: Deps) -> Uint128 {
    reward_pool_value(deps.storage)
}

pub(crate) fn query_circulating_supply(deps: Deps) -> Uint128 {
    circulating_supply(deps.storage)
}

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
// S works for both `String` and `Addr` and that's what we wanted
pub fn calculate_start_value<S: AsRef<str>>(start_after: Option<S>) -> Option<Vec<u8>> {
    start_after.as_ref().map(|identity| {
        identity
            .as_ref()
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect()
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::error::ContractError;
    use crate::mixnodes::delegation_queries::query_mixnode_delegation;
    use crate::storage::{mix_delegations};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::raw_delegation_fixture;
    use config::defaults::DENOM;
    use cosmwasm_std::coin;
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::Delegation;
    use mixnet_contract::IdentityKey;
    use mixnet_contract::RawDelegationData;

    #[test]
    fn mix_deletion_query_returns_current_delegation_value() {
        let mut deps = helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();
        let delegation_owner = Addr::unchecked("bar");

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                delegation_owner.as_bytes(),
                &RawDelegationData::new(42u128.into(), 12_345),
            )
            .unwrap();

        assert_eq!(
            Ok(Delegation::new(
                delegation_owner.clone(),
                coin(42, DENOM),
                12_345
            )),
            query_mixnode_delegation(deps.as_ref(), node_identity, delegation_owner)
        )
    }

    #[test]
    fn mix_deletion_query_returns_error_if_delegation_doesnt_exist() {
        let mut deps = helpers::init_contract();

        let node_identity1: IdentityKey = "foo1".into();
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner1 = Addr::unchecked("bar");
        let delegation_owner2 = Addr::unchecked("bar2");

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.clone(),
            }),
            query_mixnode_delegation(
                deps.as_ref(),
                node_identity1.clone(),
                delegation_owner1.clone()
            )
        );

        // add delegation from a different address
        mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner2.as_bytes(), &raw_delegation_fixture(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.clone(),
            }),
            query_mixnode_delegation(
                deps.as_ref(),
                node_identity1.clone(),
                delegation_owner1.clone()
            )
        );

        // add delegation for a different node
        mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner1.as_bytes(), &raw_delegation_fixture(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.clone()
            }),
            query_mixnode_delegation(deps.as_ref(), node_identity1.clone(), delegation_owner1)
        )
    }

    #[cfg(test)]
    mod querying_for_reverse_mixnode_delegations_paged {
        use super::*;
        use crate::mixnodes::delegation_queries::query_reverse_mixnode_delegations_paged;
        use crate::storage::reverse_mix_delegations;

        fn store_n_reverse_delegations(n: u32, storage: &mut dyn Storage, delegation_owner: &Addr) {
            for i in 0..n {
                let node_identity = format!("node{}", i);
                reverse_mix_delegations(storage, delegation_owner)
                    .save(node_identity.as_bytes(), &())
                    .unwrap();
            }
        }

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = helpers::init_contract();
            let limit = 2;
            let delegation_owner = Addr::unchecked("foo");
            store_n_reverse_delegations(100, &mut deps.storage, &delegation_owner);

            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegated_nodes.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = helpers::init_contract();
            let delegation_owner = Addr::unchecked("foo");
            store_n_reverse_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query without explicitly setting a limit
            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                None,
            )
            .unwrap();
            assert_eq!(
                DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegated_nodes.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = helpers::init_contract();
            let delegation_owner = Addr::unchecked("foo");
            store_n_reverse_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query with a crazy high limit in an attempt to use too many resources
            let crazy_limit = 1000 * DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegated_nodes.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = helpers::init_contract();
            let delegation_owner = Addr::unchecked("bar");

            reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save("1".as_bytes(), &())
                .unwrap();

            let per_page = 2;
            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegated_nodes.len());

            // save another
            reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save("2".as_bytes(), &())
                .unwrap();

            // page1 should have 2 results on it
            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegated_nodes.len());

            reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save("3".as_bytes(), &())
                .unwrap();

            // page1 still has 2 results
            let page1 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.delegated_nodes.len());

            // retrieving the next page should start after the last key on this page
            let start_after: IdentityKey = String::from("2");
            let page2 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegated_nodes.len());

            // save another one
            reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save("4".as_bytes(), &())
                .unwrap();

            let start_after = String::from("2");
            let page2 = query_reverse_mixnode_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegated_nodes.len());
        }
    }
}
