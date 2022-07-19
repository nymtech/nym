// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::constants::{
    MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT, MIXNODE_BOND_MAX_RETRIEVAL_LIMIT,
    MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT, MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT,
    UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT, UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
};
use crate::mixnodes::helpers::{get_mixnode_details_by_id, get_mixnode_details_by_owner};
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Deps, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::mixnode::{
    MixNodeBond, MixNodeDetails, MixnodeRewardingDetailsResponse, PagedMixnodesDetailsResponse,
    PagedUnbondedMixnodesResponse, StakeSaturationResponse, UnbondedMixnodeResponse,
};
use mixnet_contract_common::{
    MixOwnershipResponse, MixnodeDetailsResponse, NodeId, PagedMixnodeBondsResponse,
};

pub fn query_mixnode_bonds_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedMixnodeBondsResponse> {
    let limit = limit
        .unwrap_or(MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT)
        .min(MIXNODE_BOND_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::mixnode_bonds()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<MixNodeBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.id);

    Ok(PagedMixnodeBondsResponse::new(
        nodes,
        limit,
        start_next_after,
    ))
}

fn attach_rewarding_info(
    storage: &dyn Storage,
    read_bond: StdResult<(NodeId, MixNodeBond)>,
) -> StdResult<MixNodeDetails> {
    match read_bond {
        Ok((_, bond)) => {
            // if we managed to read the bond we MUST be able to also read rewarding information.
            // if we fail, this is a hard error and the query should definitely fail and we should investigate
            // the reasons for that.
            let mix_rewarding = rewards_storage::MIXNODE_REWARDING.load(storage, bond.id)?;
            Ok(MixNodeDetails::new(bond, mix_rewarding))
        }
        Err(err) => Err(err),
    }
}

pub fn query_mixnodes_details_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedMixnodesDetailsResponse> {
    let limit = limit
        .unwrap_or(MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT)
        .min(MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::mixnode_bonds()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| attach_rewarding_info(deps.storage, res))
        .collect::<StdResult<Vec<MixNodeDetails>>>()?;

    let start_next_after = nodes.last().map(|details| details.mix_id());

    Ok(PagedMixnodesDetailsResponse::new(
        nodes,
        limit,
        start_next_after,
    ))
}

pub fn query_unbonded_mixnodes_paged(
    deps: Deps<'_>,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedMixnodesResponse> {
    let limit = limit
        .unwrap_or(UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::UNBONDED_MIXNODES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|res| res.0);

    Ok(PagedUnbondedMixnodesResponse::new(
        nodes,
        limit,
        start_next_after,
    ))
}

pub fn query_owned_mixnode(deps: Deps<'_>, address: String) -> StdResult<MixOwnershipResponse> {
    let validated_addr = deps.api.addr_validate(&address)?;
    let mixnode_details = get_mixnode_details_by_owner(deps.storage, validated_addr.clone())?;

    Ok(MixOwnershipResponse {
        address: validated_addr,
        mixnode_details,
    })
}

pub fn query_mixnode_details(deps: Deps<'_>, mix_id: NodeId) -> StdResult<MixnodeDetailsResponse> {
    let mixnode_details = get_mixnode_details_by_id(deps.storage, mix_id)?;

    Ok(MixnodeDetailsResponse {
        mix_id,
        mixnode_details,
    })
}

pub fn query_mixnode_rewarding_details(
    deps: Deps<'_>,
    mix_id: NodeId,
) -> StdResult<MixnodeRewardingDetailsResponse> {
    let rewarding_details = rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)?;

    Ok(MixnodeRewardingDetailsResponse {
        mix_id,
        rewarding_details,
    })
}

pub fn query_unbonded_mixnode(
    deps: Deps<'_>,
    mix_id: NodeId,
) -> StdResult<UnbondedMixnodeResponse> {
    let unbonded_info = storage::UNBONDED_MIXNODES.may_load(deps.storage, mix_id)?;

    Ok(UnbondedMixnodeResponse {
        mix_id,
        unbonded_info,
    })
}

pub fn query_stake_saturation(
    deps: Deps<'_>,
    mix_id: NodeId,
) -> StdResult<StakeSaturationResponse> {
    let mix_rewarding = match rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) => mix_rewarding,
        None => {
            return Ok(StakeSaturationResponse {
                mix_id,
                current_saturation: None,
                uncapped_saturation: None,
            })
        }
    };

    let rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;

    Ok(StakeSaturationResponse {
        mix_id,
        current_saturation: Some(mix_rewarding.bond_saturation(&rewarding_params)),
        uncapped_saturation: Some(mix_rewarding.uncapped_bond_saturation(&rewarding_params)),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::interval::pending_events;
    use crate::support::tests::fixtures::good_mixnode_pledge;
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Decimal;

    #[cfg(test)]
    mod mixnode_bonds {
        use super::*;
        use crate::support::tests::fixtures::good_mixnode_pledge;

        #[test]
        fn obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            let limit = 2;

            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);
            let page1 =
                query_mixnode_bonds_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();

            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);

            // query without explicitly setting a limit
            let page1 = query_mixnode_bonds_paged(deps.as_ref(), None, None).unwrap();

            assert_eq!(
                MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 =
                query_mixnode_bonds_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(MIXNODE_BOND_MAX_RETRIEVAL_LIMIT, page1.nodes.len() as u32);
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();

            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr1",
                good_mixnode_pledge(),
            );

            let per_page = 2;
            let page1 =
                query_mixnode_bonds_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr2",
                good_mixnode_pledge(),
            );

            // page1 should have 2 results on it
            let page1 =
                query_mixnode_bonds_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr3",
                good_mixnode_pledge(),
            );

            // page1 still has the same 2 results
            let another_page1 =
                query_mixnode_bonds_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 =
                query_mixnode_bonds_paged(deps.as_ref(), Some(start_after), Option::from(per_page))
                    .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr4",
                good_mixnode_pledge(),
            );

            let page2 = query_mixnode_bonds_paged(
                deps.as_ref(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[cfg(test)]
    mod mixnode_details {
        use super::*;
        use crate::support::tests::fixtures::good_mixnode_pledge;

        #[test]
        fn obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            let limit = 2;

            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);
            let page1 =
                query_mixnodes_details_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);

            // query without explicitly setting a limit
            let page1 = query_mixnodes_details_paged(deps.as_ref(), None, None).unwrap();

            assert_eq!(
                MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            test_helpers::add_dummy_mixnodes(&mut rng, deps.as_mut(), env, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 =
                query_mixnodes_details_paged(deps.as_ref(), None, Option::from(crazy_limit))
                    .unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();

            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr1",
                good_mixnode_pledge(),
            );

            let per_page = 2;
            let page1 =
                query_mixnodes_details_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr2",
                good_mixnode_pledge(),
            );

            // page1 should have 2 results on it
            let page1 =
                query_mixnodes_details_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr3",
                good_mixnode_pledge(),
            );

            // page1 still has the same 2 results
            let another_page1 =
                query_mixnodes_details_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_mixnodes_details_paged(
                deps.as_ref(),
                Some(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            test_helpers::add_mixnode(
                &mut rng,
                deps.as_mut(),
                env.clone(),
                "addr4",
                good_mixnode_pledge(),
            );

            let page2 = query_mixnodes_details_paged(
                deps.as_ref(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[cfg(test)]
    mod unbonded_mixnodes {
        use super::*;
        use cosmwasm_std::Addr;
        use mixnet_contract_common::mixnode::UnbondedMixnode;

        #[test]
        fn obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            let limit = 2;

            test_helpers::add_dummy_unbonded_mixnodes(&mut rng, deps.as_mut(), 1000);
            let page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            test_helpers::add_dummy_unbonded_mixnodes(&mut rng, deps.as_mut(), 1000);

            // query without explicitly setting a limit
            let page1 = query_unbonded_mixnodes_paged(deps.as_ref(), None, None).unwrap();

            assert_eq!(
                UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            let mut rng = test_helpers::test_rng();
            test_helpers::add_dummy_unbonded_mixnodes(&mut rng, deps.as_mut(), 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit))
                    .unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            fn add_unbonded(storage: &mut dyn Storage, id: NodeId) {
                storage::UNBONDED_MIXNODES
                    .save(
                        storage,
                        id,
                        &UnbondedMixnode {
                            identity: format!("dummy{}", id),
                            owner: Addr::unchecked(format!("dummy{}", id)),
                            unbonding_height: 123,
                        },
                    )
                    .unwrap();
            }

            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();

            add_unbonded(deps.as_mut().storage, 1);

            let per_page = 2;
            let page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            add_unbonded(deps.as_mut().storage, 2);

            // page1 should have 2 results on it
            let page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            add_unbonded(deps.as_mut().storage, 3);

            // page1 still has the same 2 results
            let another_page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_unbonded_mixnodes_paged(
                deps.as_ref(),
                Some(start_after),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            add_unbonded(deps.as_mut().storage, 4);
            let page2 = query_unbonded_mixnodes_paged(
                deps.as_ref(),
                Option::from(start_after),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[test]
    fn query_for_owned_mixnode() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let address = "mix-owner".to_string();

        // when it doesnt exist
        let res = query_owned_mixnode(deps.as_ref(), address.clone()).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(address, res.address);

        // when it [fully] exists
        let id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            &address,
            good_mixnode_pledge(),
        );
        let res = query_owned_mixnode(deps.as_ref(), address.clone()).unwrap();
        let details = res.mixnode_details.unwrap();
        assert_eq!(address, details.bond_information.owner);
        assert_eq!(
            good_mixnode_pledge()[0],
            details.bond_information.original_pledge
        );
        assert_eq!(address, res.address);

        // when it partially exists, i.e. case when the operator unbonded, but there are still some pending delegates
        // TODO: perhaps this should work slightly differently, to return the underlying mixnode rewarding?

        // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
        let mut rewarding_details = details.rewarding_details;
        rewarding_details.delegates = Decimal::raw(12345);
        rewarding_details.unique_delegations = 1;
        rewards_storage::MIXNODE_REWARDING
            .save(deps.as_mut().storage, id, &rewarding_details)
            .unwrap();

        pending_events::unbond_mixnode(deps.as_mut(), &mock_env(), id).unwrap();
        let res = query_owned_mixnode(deps.as_ref(), address.clone()).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(address, res.address);
    }

    #[test]
    fn query_for_mixnode_details() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        // no node under this id
        let res = query_mixnode_details(deps.as_ref(), 42).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(42, res.mix_id);

        // it exists
        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            "foomp",
            good_mixnode_pledge(),
        );
        let res = query_mixnode_details(deps.as_ref(), mix_id).unwrap();
        let details = res.mixnode_details.unwrap();
        assert_eq!(mix_id, details.bond_information.id);
        assert_eq!(
            good_mixnode_pledge()[0],
            details.bond_information.original_pledge
        );
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_mixnode_rewarding_details() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        // no node under this id
        let res = query_mixnode_rewarding_details(deps.as_ref(), 42).unwrap();
        assert!(res.rewarding_details.is_none());
        assert_eq!(42, res.mix_id);

        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            "foomp",
            good_mixnode_pledge(),
        );
        let res = query_mixnode_rewarding_details(deps.as_ref(), mix_id).unwrap();
        let details = res.rewarding_details.unwrap();
        assert_eq!(
            fixtures::mix_node_cost_params_fixture(),
            details.cost_params
        );
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_unbonded_mixnode() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let sender = "mix-owner";

        // no node under this id
        let res = query_unbonded_mixnode(deps.as_ref(), 42).unwrap();
        assert!(res.unbonded_info.is_none());
        assert_eq!(42, res.mix_id);

        // add and unbond the mixnode
        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender,
            good_mixnode_pledge(),
        );
        pending_events::unbond_mixnode(deps.as_mut(), &mock_env(), mix_id).unwrap();

        let res = query_unbonded_mixnode(deps.as_ref(), mix_id).unwrap();
        assert_eq!(res.unbonded_info.unwrap().owner, sender);
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_stake_saturation() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        // no node under this id
        let res = query_stake_saturation(deps.as_ref(), 42).unwrap();
        assert!(res.current_saturation.is_none());
        assert!(res.uncapped_saturation.is_none());
        assert_eq!(42, res.mix_id);

        let rewarding_params = rewards_storage::REWARDING_PARAMS
            .load(deps.as_ref().storage)
            .unwrap();
        let saturation_point = rewarding_params.interval.stake_saturation_point;

        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            "foomp",
            good_mixnode_pledge(),
        );

        // below saturation point
        // there's only the base pledge without any delegation
        let expected =
            Decimal::from_atomics(good_mixnode_pledge()[0].amount, 0).unwrap() / saturation_point;
        let res = query_stake_saturation(deps.as_ref(), mix_id).unwrap();
        assert_eq!(expected, res.current_saturation.unwrap());
        assert_eq!(expected, res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);

        // exactly at saturation point
        let mut mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .load(deps.as_ref().storage, mix_id)
            .unwrap();
        mix_rewarding.operator = saturation_point;
        rewards_storage::MIXNODE_REWARDING
            .save(deps.as_mut().storage, mix_id, &mix_rewarding)
            .unwrap();

        let res = query_stake_saturation(deps.as_ref(), mix_id).unwrap();
        assert_eq!(Decimal::one(), res.current_saturation.unwrap());
        assert_eq!(Decimal::one(), res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);

        // above the saturation point
        let mut mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .load(deps.as_ref().storage, mix_id)
            .unwrap();
        mix_rewarding.delegates = mix_rewarding.operator * Decimal::percent(150);
        rewards_storage::MIXNODE_REWARDING
            .save(deps.as_mut().storage, mix_id, &mix_rewarding)
            .unwrap();

        let res = query_stake_saturation(deps.as_ref(), mix_id).unwrap();
        assert_eq!(Decimal::one(), res.current_saturation.unwrap());
        assert_eq!(Decimal::percent(250), res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);
    }
}
