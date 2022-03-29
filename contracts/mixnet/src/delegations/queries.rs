// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cosmwasm_std::{Api, Deps, Storage};
use cw_storage_plus::{Bound, PrimaryKey};
use mixnet_contract_common::mixnode::DelegationEvent;
use mixnet_contract_common::{
    Delegation, IdentityKey, PagedDelegatorDelegationsResponse, PagedMixDelegationsResponse,
};

pub(crate) fn query_pending_delegation_events(
    storage: &dyn Storage,
    owner_address: String,
) -> Result<Vec<DelegationEvent>, ContractError> {
    Ok(storage::PENDING_DELEGATION_EVENTS
        .sub_prefix(owner_address.as_bytes().to_vec())
        .range(storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .map(|(_key, delegation_event)| delegation_event)
        .collect::<Vec<DelegationEvent>>())
}

pub(crate) fn query_delegator_delegations_paged(
    deps: Deps<'_>,
    delegation_owner: String,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedDelegatorDelegationsResponse> {
    let validated_owner = deps.api.addr_validate(&delegation_owner)?;

    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;
    let start = start_after.map(|mix_identity| {
        Bound::ExclusiveRaw((mix_identity, validated_owner.clone()).joined_key())
    });

    let delegations = storage::delegations()
        .idx
        .owner
        .prefix(validated_owner)
        .range_raw(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = delegations
        .last()
        .map(|delegation| delegation.node_identity());

    Ok(PagedDelegatorDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

// queries for delegation value of given address for particular node
pub(crate) fn query_mixnode_delegation(
    storage: &dyn Storage,
    api: &dyn Api,
    mix_identity: IdentityKey,
    delegator: String,
) -> Result<Vec<Delegation>, ContractError> {
    let validated_delegator = api.addr_validate(&delegator)?;
    let storage_key = (
        mix_identity.clone(),
        validated_delegator.as_bytes().to_vec(),
    );

    let delegations = storage::delegations()
        .prefix(storage_key)
        .range(storage, None, None, Order::Ascending)
        .filter_map(|d| d.ok())
        .map(|r| r.1)
        .collect::<Vec<Delegation>>();

    if delegations.is_empty() {
        Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
            address: delegator,
        })
    } else {
        Ok(delegations)
    }
}

pub(crate) fn query_mixnode_delegations_paged(
    deps: Deps<'_>,
    mix_identity: IdentityKey,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
) -> StdResult<PagedMixDelegationsResponse> {
    let arg_start_after = start_after.clone();

    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;

    let start = start_after
        .map(|(addr, height)| Bound::exclusive((addr.as_bytes().to_vec(), height)));

    let delegations = storage::delegations()
        .sub_prefix(mix_identity)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|r| r.1))
        .collect::<StdResult<Vec<_>>>()?;

    let mut start_next_after = delegations
        .last()
        .map(|delegation| (delegation.owner().to_string(), delegation.block_height()));

    if arg_start_after == start_next_after {
        start_next_after = None;
    }

    Ok(PagedMixDelegationsResponse::new(
        delegations,
        start_next_after,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::{coin, Addr, Storage};
    use rand::Rng;

    pub fn store_n_mix_delegations(n: u32, storage: &mut dyn Storage, node_identity: &str) {
        for i in 0..n {
            let address = format!("address{}", i);
            test_helpers::save_dummy_delegation(storage, node_identity, address, 1);
        }
    }

    #[cfg(test)]
    mod querying_for_mixnode_delegations_paged {
        use std::collections::HashSet;

        use super::*;
        use mixnet_contract_common::IdentityKey;
        use rand::{distributions::Alphanumeric, SeedableRng};

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let limit = 2;
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(100, &mut deps.storage, &node_identity);

            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query without explicitly setting a limit
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, None).unwrap();
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            store_n_mix_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &node_identity,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * storage::DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = storage::DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let dummy_seed = [42u8; 32];
            let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();

            let mut delegation_test_data = vec![];
            let mut returned_delegation_data = HashSet::new();

            // Crete a bunch of randomly ordered (in storage) delegations
            for _ in 0..200 {
                delegation_test_data.push((
                    rng.clone()
                        .sample_iter(&Alphanumeric)
                        .take(30)
                        .map(char::from)
                        .collect::<String>(),
                    rng.gen::<u32>() as u64,
                ))
            }

            for (address, block_height) in delegation_test_data.iter() {
                test_helpers::save_dummy_delegation(
                    &mut deps.storage,
                    &node_identity,
                    address,
                    *block_height,
                );
            }

            let per_page = 100;

            // page1 still has 2 results
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();

            let start_after = page1.start_next_after.unwrap();
            assert_eq!(100, page1.delegations.len());
            assert_eq!(
                (("XtsZrLRXvyegwyZDjuJtlYiG5B1eiJ".to_string(), 1594717548)),
                start_after
            );

            for delegation in page1.delegations {
                returned_delegation_data.insert(delegation.owner().to_string());
            }

            // retrieving the next page should start after the last key on this page

            let page2 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                Option::from(start_after.clone()),
                Option::from(per_page),
            )
            .unwrap();

            let start_after = page2.start_next_after.unwrap();
            assert_eq!(
                ("zkHTlcgOWAyH8NoIJ2lkZcvvhYsFik".to_string(), 3448133410),
                start_after
            );

            for delegation in page2.delegations {
                returned_delegation_data.insert(delegation.owner().to_string());
            }

            let page3 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity.clone(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert!(page3.start_next_after.is_none());

            for delegation in delegation_test_data {
                assert!(returned_delegation_data.contains(&*delegation.0));
            }
        }
    }

    #[test]
    fn mix_deletion_query_returns_current_delegation_value() {
        let mut deps = test_helpers::init_contract();
        let node_identity: IdentityKey = "foo".into();
        let delegation_owner = Addr::unchecked("bar");

        let delegation = Delegation::new(
            delegation_owner.clone(),
            node_identity.clone(),
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(deps.as_mut().storage, delegation.storage_key(), &delegation)
            .unwrap();

        assert_eq!(
            Ok(vec![delegation]),
            query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                node_identity,
                delegation_owner.to_string()
            )
        )
    }

    #[test]
    fn mix_deletion_query_returns_error_if_delegation_doesnt_exist() {
        let mut deps = test_helpers::init_contract();

        let node_identity1: IdentityKey = "foo1".into();
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner1 = Addr::unchecked("bar");
        let delegation_owner2 = Addr::unchecked("bar2");

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.to_string(),
            }),
            query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                node_identity1.clone(),
                delegation_owner1.to_string()
            )
        );

        // add delegation from a different address
        let delegation = Delegation::new(
            delegation_owner2,
            node_identity1.clone(),
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(deps.as_mut().storage, delegation.storage_key(), &delegation)
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.to_string(),
            }),
            query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                node_identity1.clone(),
                delegation_owner1.to_string()
            )
        );

        // add delegation for a different node
        let delegation = Delegation::new(
            delegation_owner1.clone(),
            node_identity2,
            coin(1234, DENOM),
            1234,
            None,
        );

        storage::delegations()
            .save(deps.as_mut().storage, delegation.storage_key(), &delegation)
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity1.clone(),
                address: delegation_owner1.to_string()
            }),
            query_mixnode_delegation(
                &deps.storage,
                &deps.api,
                node_identity1,
                delegation_owner1.to_string()
            )
        )
    }

    #[cfg(test)]
    mod querying_for_reverse_mixnode_delegations_paged {
        use super::*;

        fn store_n_reverse_delegations(n: u32, storage: &mut dyn Storage, delegation_owner: &str) {
            for i in 0..n {
                let node_identity = format!("node{}", i);
                test_helpers::save_dummy_delegation(storage, node_identity, delegation_owner, 1);
            }
        }

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let limit = 2;
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(100, &mut deps.storage, &delegation_owner);

            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.delegations.len() as u32);
        }

        #[test]
        fn retrieval_has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query without explicitly setting a limit
            let page1 =
                query_delegator_delegations_paged(deps.as_ref(), delegation_owner, None, None)
                    .unwrap();
            assert_eq!(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "foo".to_string();
            store_n_reverse_delegations(
                storage::DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                &delegation_owner,
            );

            // query with a crazy high limit in an attempt to use too many resources
            let crazy_limit = 1000 * storage::DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = storage::DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = test_helpers::init_contract();
            let delegation_owner = "bar".to_string();

            for j in 0..20 {
                for i in 0..10 {
                    test_helpers::save_dummy_delegation(
                        &mut deps.storage,
                        format!("{}-{}", j, i),
                        delegation_owner.clone(),
                        i,
                    );
                }
            }

            let per_page = 100;
            let page1 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                None,
                Option::from(per_page),
            )
            .unwrap();

            let start_after = page1.start_next_after.unwrap();
            assert_eq!(per_page as usize, page1.delegations.len());
            assert_eq!(start_after, "9-9".to_string());

            let page2 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                Some(start_after),
                Option::from(per_page),
            )
            .unwrap();

            let start_after = page2.start_next_after.unwrap();
            assert_eq!(start_after, "19-9".to_string());

            let page3 = query_delegator_delegations_paged(
                deps.as_ref(),
                delegation_owner.clone(),
                Some(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert!(page3.start_next_after.is_none());
        }
    }
}
