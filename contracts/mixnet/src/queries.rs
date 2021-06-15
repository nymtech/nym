use crate::contract::DENOM;
use crate::error::ContractError;
use crate::state::StateParams;
use crate::storage::{
    gateway_delegations_read, gateways_owners_read, gateways_read, mix_delegations_read,
    mixnodes_owners_read, mixnodes_read, read_layer_distribution, read_state_params,
};
use cosmwasm_std::Deps;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cosmwasm_std::{coin, HumanAddr};
use mixnet_contract::{
    Delegation, GatewayBond, GatewayOwnershipResponse, IdentityStringPublicKeyWrapper,
    LayerDistribution, MixNodeBond, MixOwnershipResponse, PagedGatewayDelegationsResponse,
    PagedGatewayResponse, PagedMixDelegationsResponse, PagedResponse,
};

const BOND_PAGE_MAX_LIMIT: u32 = 100;
const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 750;
const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 500;

pub fn query_mixnodes_paged(
    deps: Deps,
    start: Option<IdentityStringPublicKeyWrapper>,
    limit: Option<u32>,
) -> StdResult<PagedResponse> {
    let limit = limit
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_MAX_LIMIT) as usize;

    // I don't like the extra allocation in here, but in the grand scheme of things of grabbing
    // hundred nodes, I guess it's fine
    let start = start.map(|key| key.to_bytes().to_vec());

    let mut nodes = mixnodes_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit + 1) // take limit + 1 elements to determine start of next page
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<MixNodeBond>>>()?;

    // if we got the additional element (which would have been on the next page)
    let start_next = if nodes.len() > limit {
        // this is the starting element of the next query
        Some(nodes[limit].identity())
    } else {
        None
    };

    // return to the caller only at most `limit` number of elements
    nodes.truncate(limit);

    Ok(PagedResponse::new(nodes, limit, start_next))
}

pub(crate) fn query_gateways_paged(
    deps: Deps,
    start: Option<IdentityStringPublicKeyWrapper>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_MAX_LIMIT) as usize;

    // I don't like the extra allocation in here, but in the grand scheme of things of grabbing
    // hundred nodes, I guess it's fine
    let start = start.map(|key| key.to_bytes().to_vec());

    let mut nodes = gateways_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit + 1) // take limit + 1 elements to determine start of next page
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    // if we got the additional element (which would have been on the next page)
    let start_next = if nodes.len() > limit {
        // this is the starting element of the next query
        Some(nodes[limit].identity())
    } else {
        None
    };

    // return to the caller only at most `limit` number of elements
    nodes.truncate(limit);

    Ok(PagedGatewayResponse::new(nodes, limit, start_next))
}

pub(crate) fn query_owns_mixnode(
    deps: Deps,
    address: HumanAddr,
) -> StdResult<MixOwnershipResponse> {
    let has_node = mixnodes_owners_read(deps.storage)
        .may_load(address.as_ref())?
        .is_some();
    Ok(MixOwnershipResponse { address, has_node })
}

pub(crate) fn query_owns_gateway(
    deps: Deps,
    address: HumanAddr,
) -> StdResult<GatewayOwnershipResponse> {
    let has_gateway = gateways_owners_read(deps.storage)
        .may_load(address.as_ref())?
        .is_some();
    Ok(GatewayOwnershipResponse {
        address,
        has_gateway,
    })
}

pub(crate) fn query_state_params(deps: Deps) -> StateParams {
    read_state_params(deps.storage)
}

pub(crate) fn query_layer_distribution(deps: Deps) -> LayerDistribution {
    read_layer_distribution(deps.storage)
}

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
fn calculate_start_value(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    start_after.as_ref().map(|identity| {
        identity
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect()
    })
}

pub(crate) fn query_mixnode_delegations_paged(
    deps: Deps,
    mix_identity: IdentityStringPublicKeyWrapper,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<PagedMixDelegationsResponse> {
    let limit = limit
        .unwrap_or(DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(DELEGATION_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let delegations = mix_delegations_read(deps.storage, mix_identity)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|entry| {
                Delegation::new(
                    String::from_utf8(entry.0).unwrap().into(),
                    coin(entry.1.u128(), DENOM),
                )
            })
        })
        .collect::<StdResult<Vec<Delegation>>>()?;

    let start_next_after = delegations
        .last()
        .map(|delegation| delegation.owner().clone());

    Ok(PagedMixDelegationsResponse::new(
        mix_identity,
        delegations,
        start_next_after,
    ))
}

// queries for delegation value of given address for particular node
pub(crate) fn query_mixnode_delegation(
    deps: Deps,
    mix_identity: IdentityStringPublicKeyWrapper,
    address: HumanAddr,
) -> Result<Delegation, ContractError> {
    match mix_delegations_read(deps.storage, mix_identity).may_load(address.as_bytes())? {
        Some(delegation_value) => Ok(Delegation::new(
            address,
            coin(delegation_value.u128(), DENOM),
        )),
        None => Err(ContractError::NoMixnodeDelegationFound {
            identity: mix_identity,
        }),
    }
}

pub(crate) fn query_gateway_delegations_paged(
    deps: Deps,
    gateway_identity: IdentityStringPublicKeyWrapper,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayDelegationsResponse> {
    let limit = limit
        .unwrap_or(DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(DELEGATION_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let delegations = gateway_delegations_read(deps.storage, gateway_identity)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|entry| {
                Delegation::new(
                    String::from_utf8(entry.0).unwrap().into(),
                    coin(entry.1.u128(), DENOM),
                )
            })
        })
        .collect::<StdResult<Vec<Delegation>>>()?;

    let start_next_after = delegations
        .last()
        .map(|delegation| delegation.owner().clone());

    Ok(PagedGatewayDelegationsResponse::new(
        gateway_identity,
        delegations,
        start_next_after,
    ))
}

// queries for delegation value of given address for particular node
pub(crate) fn query_gateway_delegation(
    deps: Deps,
    gateway_identity: IdentityStringPublicKeyWrapper,
    address: HumanAddr,
) -> Result<Delegation, ContractError> {
    match gateway_delegations_read(deps.storage, gateway_identity).may_load(address.as_bytes())? {
        Some(delegation_value) => Ok(Delegation::new(
            address,
            coin(delegation_value.u128(), DENOM),
        )),
        None => Err(ContractError::NoGatewayDelegationFound {
            identity: gateway_identity,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use crate::storage::{config, gateway_delegations, gateways, mix_delegations, mixnodes};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{good_gateway_bond, good_mixnode_bond, HashToIdentity};
    use crate::transactions;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Storage, Uint128};
    use mixnet_contract::{Gateway, MixNode};

    #[test]
    fn mixnodes_empty_on_init() {
        let deps = helpers::init_contract();
        let response = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn mixnodes_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..100 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();

        let expected_limit = 50;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = 100;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn pagination_works() {
        let mut deps = helpers::init_contract();
        let node = helpers::mixnode_bond_fixture();

        // generate four identities and sort them by their bytes representation to simulate ordered retrieval
        let mut identities = vec![
            "1".hash_to_identity(),
            "2".hash_to_identity(),
            "3".hash_to_identity(),
            "4".hash_to_identity(),
        ];
        identities.sort_by(|a, b| a.to_bytes().cmp(&b.to_bytes()));
        let one = identities[0];
        let two = identities[1];
        let three = identities[2];
        let four = identities[3];

        mixnodes(&mut deps.storage)
            .save(one.as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        mixnodes(&mut deps.storage)
            .save(two.as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        mixnodes(&mut deps.storage)
            .save(three.as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start with the first key on the next page
        let start = Some(three);
        let page2 = query_mixnodes_paged(deps.as_ref(), start, Some(per_page)).unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        mixnodes(&mut deps.storage)
            .save(four.as_bytes(), &node)
            .unwrap();

        let start = Some(three);
        let page2 = query_mixnodes_paged(deps.as_ref(), start, Some(per_page)).unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.nodes.len());
    }

    #[test]
    fn gateways_empty_on_init() {
        let deps = helpers::init_contract();
        let response = query_gateways_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    fn store_n_gateway_fixtures(n: u32, storage: &mut dyn Storage) {
        for i in 0..n {
            let key = format!("bond{}", i);
            let node = helpers::gateway_bond_fixture();
            gateways(storage).save(key.as_bytes(), &node).unwrap();
        }
    }

    #[test]
    fn gateways_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        store_n_gateway_fixtures(100, storage);

        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(10 * BOND_PAGE_DEFAULT_LIMIT, storage);

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(100, storage);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * BOND_PAGE_DEFAULT_LIMIT;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = BOND_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        let mut deps = helpers::init_contract();
        let node = helpers::gateway_bond_fixture();

        // generate four identities and sort them by their bytes representation to simulate ordered retrieval
        let mut identities = vec![
            "1".hash_to_identity(),
            "2".hash_to_identity(),
            "3".hash_to_identity(),
            "4".hash_to_identity(),
        ];
        identities.sort_by(|a, b| a.to_bytes().cmp(&b.to_bytes()));
        let one = identities[0];
        let two = identities[1];
        let three = identities[2];
        let four = identities[3];

        gateways(&mut deps.storage)
            .save(one.as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_gateways_paged(deps.as_ref(), None, Some(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        gateways(&mut deps.storage)
            .save(two.as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(deps.as_ref(), None, Some(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        gateways(&mut deps.storage)
            .save(three.as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_gateways_paged(deps.as_ref(), None, Some(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start with the first key on the next page
        let start = Some(three);
        let page2 = query_gateways_paged(deps.as_ref(), start, Some(per_page)).unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        gateways(&mut deps.storage)
            .save(four.as_bytes(), &node)
            .unwrap();

        let start = Some(three);
        let page2 = query_gateways_paged(deps.as_ref(), start, Some(per_page)).unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.nodes.len());
    }

    #[test]
    fn query_for_mixnode_owner_works() {
        let mut deps = helpers::init_contract();
        let env = mock_env();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_node);

        // mixnode was added to "bob", "fred" still does not own one
        let node = MixNode {
            identity_key: "bobsnode".hash_to_identity(),
            ..helpers::mix_node_fixture()
        };
        transactions::try_add_mixnode(deps.as_mut(), mock_info("bob", &good_mixnode_bond()), node)
            .unwrap();
        //
        // let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        // assert!(!res.has_node);
        //
        // // "fred" now owns a mixnode!
        // let node = MixNode {
        //     identity_key: "fredsnode".hash_to_identity(),
        //     ..helpers::mix_node_fixture()
        // };
        // transactions::try_add_mixnode(deps.as_mut(), mock_info("fred", &good_mixnode_bond()), node)
        //     .unwrap();
        //
        // let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        // assert!(res.has_node);
        //
        // // but after unbonding it, he doesn't own one anymore
        // transactions::try_remove_mixnode(deps.as_mut(), mock_info("fred", &[]), env).unwrap();
        //
        // let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        // assert!(!res.has_node);
    }

    #[test]
    fn query_for_gateway_owner_works() {
        let mut deps = helpers::init_contract();
        let env = mock_env();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);

        // mixnode was added to "bob", "fred" still does not own one
        let node = Gateway {
            identity_key: "bobsnode".hash_to_identity(),
            ..helpers::gateway_fixture()
        };
        transactions::try_add_gateway(deps.as_mut(), mock_info("bob", &good_gateway_bond()), node)
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);

        // "fred" now owns a gateway!
        let node = Gateway {
            identity_key: "fredsnode".hash_to_identity(),
            ..helpers::gateway_fixture()
        };
        transactions::try_add_gateway(deps.as_mut(), mock_info("fred", &good_gateway_bond()), node)
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(res.has_gateway);

        // but after unbonding it, he doesn't own one anymore
        transactions::try_remove_gateway(deps.as_mut(), mock_info("fred", &[]), env).unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);
    }

    #[test]
    fn query_for_contract_state_works() {
        let mut deps = helpers::init_contract();

        let dummy_state = State {
            owner: "someowner".into(),
            network_monitor_address: "monitor".into(),
            params: StateParams {
                epoch_length: 1,
                minimum_mixnode_bond: 123u128.into(),
                minimum_gateway_bond: 456u128.into(),
                mixnode_bond_reward_rate: "1.23".parse().unwrap(),
                gateway_bond_reward_rate: "4.56".parse().unwrap(),
                mixnode_active_set_size: 1000,
            },
            mixnode_epoch_bond_reward: "1.23".parse().unwrap(),
            gateway_epoch_bond_reward: "4.56".parse().unwrap(),
        };

        config(deps.as_mut().storage).save(&dummy_state).unwrap();

        assert_eq!(dummy_state.params, query_state_params(deps.as_ref()))
    }

    #[cfg(test)]
    mod querying_for_mixnode_delegations_paged {
        use super::*;
        use crate::storage::mix_delegations;
        use crate::support::tests::helpers::HashToIdentity;
        use cosmwasm_std::Uint128;

        fn store_n_delegations(
            n: u32,
            storage: &mut dyn Storage,
            node_identity: IdentityStringPublicKeyWrapper,
        ) {
            for i in 0..n {
                let address = format!("address{}", i);
                mix_delegations(storage, node_identity)
                    .save(address.as_bytes(), &Uint128(42))
                    .unwrap();
            }
        }

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = helpers::init_contract();
            let limit = 2;
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(100, &mut deps.storage, node_identity);

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
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                node_identity,
            );

            // query without explicitly setting a limit
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, None).unwrap();
            assert_eq!(
                DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                node_identity,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();

            mix_delegations(&mut deps.storage, node_identity)
                .save("1".as_bytes(), &Uint128(42))
                .unwrap();

            let per_page = 2;
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            mix_delegations(&mut deps.storage, node_identity)
                .save("2".as_bytes(), &Uint128(42))
                .unwrap();

            // page1 should have 2 results on it
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            mix_delegations(&mut deps.storage, node_identity)
                .save("3".as_bytes(), &Uint128(42))
                .unwrap();

            // page1 still has 2 results
            let page1 =
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            // retrieving the next page should start after the last key on this page
            let start_after = Some(HumanAddr::from("2"));
            let page2 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                start_after,
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            mix_delegations(&mut deps.storage, node_identity)
                .save("4".as_bytes(), &Uint128(42))
                .unwrap();

            let start_after = Some(HumanAddr::from("2"));
            let page2 = query_mixnode_delegations_paged(
                deps.as_ref(),
                node_identity,
                start_after,
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }
    }

    #[test]
    fn mix_deletion_query_returns_current_delegation_value() {
        let mut deps = helpers::init_contract();
        let node_identity = "foo".hash_to_identity();
        let delegator_address: HumanAddr = "bar".into();

        mix_delegations(&mut deps.storage, node_identity)
            .save(delegator_address.as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Ok(Delegation::new(delegator_address.clone(), coin(42, DENOM))),
            query_mixnode_delegation(deps.as_ref(), node_identity, delegator_address)
        )
    }

    #[test]
    fn mix_deletion_query_returns_error_if_delegation_doesnt_exist() {
        let mut deps = helpers::init_contract();
        let node_identity = "foo".hash_to_identity();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity,
            }),
            query_mixnode_delegation(deps.as_ref(), node_identity, "foo".into())
        );

        // add delegation from a different address
        mix_delegations(&mut deps.storage, node_identity)
            .save("bar".as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity,
            }),
            query_mixnode_delegation(deps.as_ref(), node_identity, "foo".into())
        );

        // add delegation for a different node
        mix_delegations(&mut deps.storage, "differentnode".hash_to_identity())
            .save("foo".as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoMixnodeDelegationFound {
                identity: node_identity,
            }),
            query_mixnode_delegation(deps.as_ref(), node_identity, "foo".into())
        )
    }

    #[cfg(test)]
    mod querying_for_gateway_delegations_paged {
        use super::*;
        use crate::storage::gateway_delegations;
        use cosmwasm_std::Uint128;

        fn store_n_delegations(
            n: u32,
            storage: &mut dyn Storage,
            node_identity: IdentityStringPublicKeyWrapper,
        ) {
            for i in 0..n {
                let address = format!("address{}", i);
                gateway_delegations(storage, node_identity)
                    .save(address.as_bytes(), &Uint128(42))
                    .unwrap();
            }
        }

        #[test]
        fn retrieval_obeys_limits() {
            let mut deps = helpers::init_contract();
            let limit = 2;
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(100, &mut deps.storage, node_identity);

            let page1 = query_gateway_delegations_paged(
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
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                node_identity,
            );

            // query without explicitly setting a limit
            let page1 =
                query_gateway_delegations_paged(deps.as_ref(), node_identity, None, None).unwrap();
            assert_eq!(
                DELEGATION_PAGE_DEFAULT_LIMIT,
                page1.delegations.len() as u32
            );
        }

        #[test]
        fn retrieval_has_max_limit() {
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();
            store_n_delegations(
                DELEGATION_PAGE_DEFAULT_LIMIT * 10,
                &mut deps.storage,
                node_identity,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * DELEGATION_PAGE_DEFAULT_LIMIT;
            let page1 = query_gateway_delegations_paged(
                deps.as_ref(),
                node_identity,
                None,
                Option::from(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DELEGATION_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.delegations.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut deps = helpers::init_contract();
            let node_identity = "foo".hash_to_identity();

            gateway_delegations(&mut deps.storage, node_identity)
                .save("1".as_bytes(), &Uint128(42))
                .unwrap();

            let per_page = 2;
            let page1 =
                query_gateway_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.delegations.len());

            // save another
            gateway_delegations(&mut deps.storage, node_identity)
                .save("2".as_bytes(), &Uint128(42))
                .unwrap();

            // page1 should have 2 results on it
            let page1 =
                query_gateway_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            gateway_delegations(&mut deps.storage, node_identity)
                .save("3".as_bytes(), &Uint128(42))
                .unwrap();

            // page1 still has 2 results
            let page1 =
                query_gateway_delegations_paged(deps.as_ref(), node_identity, None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.delegations.len());

            // retrieving the next page should start after the last key on this page
            let start_after = Some(HumanAddr::from("2"));
            let page2 = query_gateway_delegations_paged(
                deps.as_ref(),
                node_identity,
                start_after,
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.delegations.len());

            // save another one
            gateway_delegations(&mut deps.storage, node_identity)
                .save("4".as_bytes(), &Uint128(42))
                .unwrap();

            let start_after = Some(HumanAddr::from("2"));
            let page2 = query_gateway_delegations_paged(
                deps.as_ref(),
                node_identity,
                start_after,
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.delegations.len());
        }
    }

    #[test]
    fn gateway_deletion_query_returns_current_delegation_value() {
        let mut deps = helpers::init_contract();
        let node_identity = "foo".hash_to_identity();
        let delegator_address: HumanAddr = "bar".into();

        gateway_delegations(&mut deps.storage, node_identity)
            .save(delegator_address.as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Ok(Delegation::new(delegator_address.clone(), coin(42, DENOM))),
            query_gateway_delegation(deps.as_ref(), node_identity, delegator_address)
        )
    }

    #[test]
    fn gateway_deletion_query_returns_error_if_delegation_doesnt_exist() {
        let mut deps = helpers::init_contract();
        let node_identity = "foo".hash_to_identity();

        assert_eq!(
            Err(ContractError::NoGatewayDelegationFound {
                identity: node_identity,
            }),
            query_gateway_delegation(deps.as_ref(), node_identity, "foo".into())
        );

        // add delegation from a different address
        gateway_delegations(&mut deps.storage, node_identity)
            .save("bar".as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoGatewayDelegationFound {
                identity: node_identity,
            }),
            query_gateway_delegation(deps.as_ref(), node_identity, "foo".into())
        );

        // add delegation for a different node
        gateway_delegations(&mut deps.storage, "differentnode".hash_to_identity())
            .save("foo".as_bytes(), &Uint128(42))
            .unwrap();

        assert_eq!(
            Err(ContractError::NoGatewayDelegationFound {
                identity: node_identity,
            }),
            query_gateway_delegation(deps.as_ref(), node_identity, "foo".into())
        )
    }
}
