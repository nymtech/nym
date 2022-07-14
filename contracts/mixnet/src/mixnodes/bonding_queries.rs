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
        None => return Ok(StakeSaturationResponse::default()),
    };

    let rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;

    Ok(StakeSaturationResponse {
        current_saturation: Some(mix_rewarding.bond_saturation(&rewarding_params)),
        uncapped_saturation: Some(mix_rewarding.uncapped_bond_saturation(&rewarding_params)),
    })
}

//
// #[cfg(test)]
// pub(crate) mod tests {
//     use super::storage;
//     use super::*;
//     use crate::mixnodes::storage::BOND_PAGE_DEFAULT_LIMIT;
//     use crate::support::tests::test_helpers;
//     use crate::{contract::execute, support::tests};
//     use cosmwasm_std::testing::{mock_env, mock_info};
//
//     #[test]
//     fn mixnodes_empty_on_init() {
//         let deps = test_helpers::init_contract();
//         let response = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
//         assert_eq!(0, response.nodes.len());
//     }
//
//     #[test]
//     fn mixnodes_paged_retrieval_obeys_limits() {
//         let mut deps = test_helpers::init_contract();
//         let limit = 2;
//         for n in 0..1000 {
//             let key = format!("bond{}", n);
//             test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//         }
//
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
//         assert_eq!(limit, page1.nodes.len() as u32);
//     }
//
//     #[test]
//     fn mixnodes_paged_retrieval_has_default_limit() {
//         let mut deps = test_helpers::init_contract();
//         for n in 0..1000 {
//             let key = format!("bond{}", n);
//             test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//         }
//
//         // query without explicitly setting a limit
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();
//
//         assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
//     }
//
//     #[test]
//     fn mixnodes_paged_retrieval_has_max_limit() {
//         let mut deps = test_helpers::init_contract();
//         for n in 0..1000 {
//             let key = format!("bond{}", n);
//             test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//         }
//
//         // query with a crazily high limit in an attempt to use too many resources
//         let crazy_limit = 1000;
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();
//
//         // we default to a decent sized upper bound instead
//         let expected_limit = storage::BOND_PAGE_MAX_LIMIT;
//         assert_eq!(expected_limit, page1.nodes.len() as u32);
//     }
//
//     #[test]
//     fn pagination_works() {
//         // prepare 4 messages and identities that are sorted by the generated identities
//         // (because we query them in an ascended manner)
//         let mut exec_data = (0..4)
//             .map(|i| {
//                 let sender = format!("nym-addr{}", i);
//                 let (msg, identity) = tests::messages::valid_bond_mixnode_msg(&sender);
//                 (msg, (sender, identity))
//             })
//             .collect::<Vec<_>>();
//         exec_data.sort_by(|(_, (_, id1)), (_, (_, id2))| id1.cmp(id2));
//         let (messages, sender_identities): (Vec<_>, Vec<_>) = exec_data.into_iter().unzip();
//
//         let mut deps = test_helpers::init_contract();
//
//         let info = mock_info(
//             &sender_identities[0].0.clone(),
//             &tests::fixtures::good_mixnode_pledge(),
//         );
//         execute(deps.as_mut(), mock_env(), info, messages[0].clone()).unwrap();
//
//         let per_page = 2;
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
//
//         // page should have 1 result on it
//         assert_eq!(1, page1.nodes.len());
//
//         // save another
//         let info = mock_info(
//             &sender_identities[1].0.clone(),
//             &tests::fixtures::good_mixnode_pledge(),
//         );
//         execute(deps.as_mut(), mock_env(), info, messages[1].clone()).unwrap();
//
//         // page1 should have 2 results on it
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
//         assert_eq!(2, page1.nodes.len());
//
//         let info = mock_info(
//             &sender_identities[2].0.clone(),
//             &tests::fixtures::good_mixnode_pledge(),
//         );
//         execute(deps.as_mut(), mock_env(), info, messages[2].clone()).unwrap();
//
//         // page1 still has 2 results
//         let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
//         assert_eq!(2, page1.nodes.len());
//
//         // retrieving the next page should start after the last key on this page
//         let start_after = page1.start_next_after.unwrap();
//         let page2 = query_mixnodes_paged(
//             deps.as_ref(),
//             Option::from(start_after.clone()),
//             Option::from(per_page),
//         )
//         .unwrap();
//
//         assert_eq!(1, page2.nodes.len());
//
//         // save another one
//         let info = mock_info(
//             &sender_identities[3].0.clone(),
//             &tests::fixtures::good_mixnode_pledge(),
//         );
//         execute(deps.as_mut(), mock_env(), info, messages[3].clone()).unwrap();
//
//         let page2 = query_mixnodes_paged(
//             deps.as_ref(),
//             Option::from(start_after),
//             Option::from(per_page),
//         )
//         .unwrap();
//
//         // now we have 2 pages, with 2 results on the second page
//         assert_eq!(2, page2.nodes.len());
//     }
//
//     #[test]
//     fn query_for_mixnode_owner_works() {
//         let mut deps = test_helpers::init_contract();
//         let env = mock_env();
//
//         // "fred" does not own a mixnode if there are no mixnodes
//         let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
//         assert!(res.mixnode.is_none());
//
//         // mixnode was added to "bob", "fred" still does not own one
//         test_helpers::add_mixnode("bob", tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//
//         let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
//         assert!(res.mixnode.is_none());
//
//         // "fred" now owns a mixnode!
//         test_helpers::add_mixnode(
//             "fred",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
//         assert!(res.mixnode.is_some());
//
//         // but after unbonding it, he doesn't own one anymore
//         crate::mixnodes::transactions::try_remove_mixnode(
//             env,
//             deps.as_mut(),
//             mock_info("fred", &[]),
//         )
//         .unwrap();
//
//         let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
//         assert!(res.mixnode.is_none());
//     }
// }
