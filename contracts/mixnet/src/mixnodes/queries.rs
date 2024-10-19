// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::constants::{
    MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT, MIXNODE_BOND_MAX_RETRIEVAL_LIMIT,
    MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT, MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT,
    UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT, UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
};
use crate::mixnodes::helpers::{
    attach_mix_details, get_mixnode_details_by_id, get_mixnode_details_by_identity,
    get_mixnode_details_by_owner,
};
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Deps, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::mixnode::{
    MixNodeBond, MixNodeDetails, MixStakeSaturationResponse, MixnodeRewardingDetailsResponse,
    PagedMixnodesDetailsResponse, PagedUnbondedMixnodesResponse, UnbondedMixnodeResponse,
};
use mixnet_contract_common::{
    IdentityKey, MixOwnershipResponse, MixnodeDetailsByIdentityResponse, MixnodeDetailsResponse,
    NodeId, PagedMixnodeBondsResponse,
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

    let start_next_after = nodes.last().map(|node| node.mix_id);

    Ok(PagedMixnodeBondsResponse::new(
        nodes,
        limit,
        start_next_after,
    ))
}

fn attach_node_details(
    storage: &dyn Storage,
    read_bond: StdResult<(NodeId, MixNodeBond)>,
) -> StdResult<MixNodeDetails> {
    match read_bond {
        Ok((_, bond)) => attach_mix_details(storage, bond),
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
        .map(|res| attach_node_details(deps.storage, res))
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

    let nodes = storage::unbonded_mixnodes()
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

pub fn query_unbonded_mixnodes_by_owner_paged(
    deps: Deps<'_>,
    owner: String,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedMixnodesResponse> {
    let owner = deps.api.addr_validate(&owner)?;

    let limit = limit
        .unwrap_or(UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::unbonded_mixnodes()
        .idx
        .owner
        .prefix(owner)
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

pub fn query_unbonded_mixnodes_by_identity_paged(
    deps: Deps<'_>,
    identity_key: String,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> StdResult<PagedUnbondedMixnodesResponse> {
    let limit = limit
        .unwrap_or(UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT)
        .min(UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::unbonded_mixnodes()
        .idx
        .identity_key
        .prefix(identity_key)
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

pub fn query_mixnode_details_by_identity(
    deps: Deps<'_>,
    identity_key: IdentityKey,
) -> StdResult<MixnodeDetailsByIdentityResponse> {
    let mixnode_details = get_mixnode_details_by_identity(deps.storage, identity_key.clone())?;

    Ok(MixnodeDetailsByIdentityResponse {
        identity_key,
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
    let unbonded_info = storage::unbonded_mixnodes().may_load(deps.storage, mix_id)?;

    Ok(UnbondedMixnodeResponse {
        mix_id,
        unbonded_info,
    })
}

pub fn query_stake_saturation(
    deps: Deps<'_>,
    mix_id: NodeId,
) -> StdResult<MixStakeSaturationResponse> {
    let mix_rewarding = match rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) => mix_rewarding,
        None => {
            return Ok(MixStakeSaturationResponse {
                mix_id,
                current_saturation: None,
                uncapped_saturation: None,
            })
        }
    };

    let rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;

    Ok(MixStakeSaturationResponse {
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
    use crate::support::tests::test_helpers::TestSetup;
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Decimal;

    #[cfg(test)]
    mod mixnode_bonds {
        use super::*;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);
            let limit = 2;

            let page1 = query_mixnode_bonds_paged(test.deps(), None, Some(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);

            // query without explicitly setting a limit
            let page1 = query_mixnode_bonds_paged(test.deps(), None, None).unwrap();

            assert_eq!(
                MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_mixnode_bonds_paged(test.deps(), None, Some(crazy_limit)).unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(MIXNODE_BOND_MAX_RETRIEVAL_LIMIT, page1.nodes.len() as u32);
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut test = TestSetup::new();

            test.add_legacy_mixnode("addr1", None);

            let per_page = 2;
            let page1 = query_mixnode_bonds_paged(test.deps(), None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            test.add_legacy_mixnode("addr2", None);

            // page1 should have 2 results on it
            let page1 = query_mixnode_bonds_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            test.add_legacy_mixnode("addr3", None);

            // page1 still has the same 2 results
            let another_page1 =
                query_mixnode_bonds_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 =
                query_mixnode_bonds_paged(test.deps(), Some(start_after), Some(per_page)).unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            test.add_legacy_mixnode("addr4", None);

            let page2 =
                query_mixnode_bonds_paged(test.deps(), Some(start_after), Some(per_page)).unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[cfg(test)]
    mod mixnode_details {
        use super::*;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);
            let limit = 2;

            let page1 = query_mixnodes_details_paged(test.deps(), None, Some(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);

            // query without explicitly setting a limit
            let page1 = query_mixnodes_details_paged(test.deps(), None, None).unwrap();

            assert_eq!(
                MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnodes(1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_mixnodes_details_paged(test.deps(), None, Some(crazy_limit)).unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut test = TestSetup::new();

            test.add_legacy_mixnode("addr1", None);

            let per_page = 2;
            let page1 = query_mixnodes_details_paged(test.deps(), None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            test.add_legacy_mixnode("addr2", None);

            // page1 should have 2 results on it
            let page1 = query_mixnodes_details_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            test.add_legacy_mixnode("addr3", None);

            // page1 still has the same 2 results
            let another_page1 =
                query_mixnodes_details_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 =
                query_mixnodes_details_paged(test.deps(), Some(start_after), Some(per_page))
                    .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            test.add_legacy_mixnode("addr4", None);

            let page2 =
                query_mixnodes_details_paged(test.deps(), Some(start_after), Some(per_page))
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
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let limit = 2;

            test_helpers::add_dummy_unbonded_mixnodes(rng, deps.as_mut(), 1000);
            let page1 = query_unbonded_mixnodes_paged(deps.as_ref(), None, Some(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            test_helpers::add_dummy_unbonded_mixnodes(rng, deps.as_mut(), 1000);

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
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            test_helpers::add_dummy_unbonded_mixnodes(rng, deps.as_mut(), 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Some(crazy_limit)).unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            fn add_unbonded(storage: &mut dyn Storage, id: NodeId) {
                storage::unbonded_mixnodes()
                    .save(
                        storage,
                        id,
                        &UnbondedMixnode {
                            identity_key: format!("dummy{}", id),
                            owner: Addr::unchecked(format!("dummy{}", id)),
                            proxy: None,
                            unbonding_height: 123,
                        },
                    )
                    .unwrap();
            }

            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();

            add_unbonded(deps.as_mut().storage, 1);

            let per_page = 2;
            let page1 = query_unbonded_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            add_unbonded(deps.as_mut().storage, 2);

            // page1 should have 2 results on it
            let page1 = query_unbonded_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            add_unbonded(deps.as_mut().storage, 3);

            // page1 still has the same 2 results
            let another_page1 =
                query_unbonded_mixnodes_paged(deps.as_ref(), None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 =
                query_unbonded_mixnodes_paged(deps.as_ref(), Some(start_after), Some(per_page))
                    .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            add_unbonded(deps.as_mut().storage, 4);
            let page2 =
                query_unbonded_mixnodes_paged(deps.as_ref(), Some(start_after), Some(per_page))
                    .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[cfg(test)]
    mod unbonded_mixnodes_by_owner {
        use super::*;
        use cosmwasm_std::Addr;
        use mixnet_contract_common::mixnode::UnbondedMixnode;

        fn add_unbonded_with_owner(storage: &mut dyn Storage, id: NodeId, owner: &str) {
            storage::unbonded_mixnodes()
                .save(
                    storage,
                    id,
                    &UnbondedMixnode {
                        identity_key: format!("dummy{}", id),
                        owner: Addr::unchecked(owner),
                        proxy: None,
                        unbonding_height: 123,
                    },
                )
                .unwrap();
        }

        #[test]
        fn obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let limit = 2;
            let owner = "owner";

            test_helpers::add_dummy_unbonded_mixnodes_with_owner(rng, deps.as_mut(), owner, 1000);
            let page1 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                None,
                Some(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let owner = "owner";

            test_helpers::add_dummy_unbonded_mixnodes_with_owner(rng, deps.as_mut(), owner, 1000);

            // query without explicitly setting a limit
            let page1 =
                query_unbonded_mixnodes_by_owner_paged(deps.as_ref(), owner.into(), None, None)
                    .unwrap();

            assert_eq!(
                UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let owner = "owner";

            test_helpers::add_dummy_unbonded_mixnodes_with_owner(rng, deps.as_mut(), owner, 1000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                None,
                Some(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();
            let owner = "owner";
            add_unbonded_with_owner(deps.as_mut().storage, 1, owner);

            let per_page = 2;
            let page1 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                None,
                Some(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            add_unbonded_with_owner(deps.as_mut().storage, 2, owner);

            // page1 should have 2 results on it
            let page1 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.nodes.len());

            add_unbonded_with_owner(deps.as_mut().storage, 3, owner);

            // page1 still has the same 2 results
            let another_page1 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            add_unbonded_with_owner(deps.as_mut().storage, 4, owner);
            let page2 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                owner.into(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }

        #[test]
        fn only_retrieves_nodes_with_specific_owner() {
            let mut deps = test_helpers::init_contract();
            let owner1 = "owner1";
            let owner2 = "owner2";
            let owner3 = "owner3";
            let owner4 = "owner4";

            add_unbonded_with_owner(deps.as_mut().storage, 1, owner1);
            add_unbonded_with_owner(deps.as_mut().storage, 2, owner1);
            add_unbonded_with_owner(deps.as_mut().storage, 3, owner2);
            add_unbonded_with_owner(deps.as_mut().storage, 4, owner1);
            add_unbonded_with_owner(deps.as_mut().storage, 5, owner3);
            add_unbonded_with_owner(deps.as_mut().storage, 6, owner3);
            add_unbonded_with_owner(deps.as_mut().storage, 7, owner4);
            add_unbonded_with_owner(deps.as_mut().storage, 8, owner2);
            add_unbonded_with_owner(deps.as_mut().storage, 9, owner1);
            add_unbonded_with_owner(deps.as_mut().storage, 10, owner3);

            let expected_ids1 = vec![1, 2, 4, 9];
            let expected_ids2 = vec![3, 8];
            let expected_ids3 = vec![5, 6, 10];
            let expected_ids4 = vec![7];

            let res1 =
                query_unbonded_mixnodes_by_owner_paged(deps.as_ref(), owner1.into(), None, None)
                    .unwrap()
                    .nodes
                    .into_iter()
                    .map(|r| r.0)
                    .collect::<Vec<_>>();
            assert_eq!(res1, expected_ids1);

            let res2 =
                query_unbonded_mixnodes_by_owner_paged(deps.as_ref(), owner2.into(), None, None)
                    .unwrap()
                    .nodes
                    .into_iter()
                    .map(|r| r.0)
                    .collect::<Vec<_>>();
            assert_eq!(res2, expected_ids2);

            let res3 =
                query_unbonded_mixnodes_by_owner_paged(deps.as_ref(), owner3.into(), None, None)
                    .unwrap()
                    .nodes
                    .into_iter()
                    .map(|r| r.0)
                    .collect::<Vec<_>>();
            assert_eq!(res3, expected_ids3);

            let res4 =
                query_unbonded_mixnodes_by_owner_paged(deps.as_ref(), owner4.into(), None, None)
                    .unwrap()
                    .nodes
                    .into_iter()
                    .map(|r| r.0)
                    .collect::<Vec<_>>();
            assert_eq!(res4, expected_ids4);

            let res5 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                "doesnt-exist".into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert!(res5.is_empty());
        }
    }

    #[cfg(test)]
    mod unbonded_mixnodes_by_identity {
        use super::*;
        use cosmwasm_std::Addr;
        use mixnet_contract_common::mixnode::UnbondedMixnode;

        fn add_unbonded_with_identity(storage: &mut dyn Storage, id: NodeId, identity: &str) {
            storage::unbonded_mixnodes()
                .save(
                    storage,
                    id,
                    &UnbondedMixnode {
                        identity_key: identity.to_string(),
                        owner: Addr::unchecked(format!("dummy{}", id)),
                        proxy: None,
                        unbonding_height: 123,
                    },
                )
                .unwrap();
        }

        #[test]
        fn obeys_limits() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let limit = 2;
            let identity = "foomp123";

            test_helpers::add_dummy_unbonded_mixnodes_with_identity(
                rng,
                deps.as_mut(),
                identity,
                1000,
            );
            let page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                Some(limit),
            )
            .unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let identity = "foomp123";
            test_helpers::add_dummy_unbonded_mixnodes_with_identity(
                rng,
                deps.as_mut(),
                identity,
                1000,
            );

            // query without explicitly setting a limit
            let page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                None,
            )
            .unwrap();

            assert_eq!(
                UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut deps = test_helpers::init_contract();
            let _env = mock_env();
            let rng = test_helpers::test_rng();
            let identity = "foomp123";
            test_helpers::add_dummy_unbonded_mixnodes_with_identity(
                rng,
                deps.as_mut(),
                identity,
                1000,
            );

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000;
            let page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                Some(crazy_limit),
            )
            .unwrap();

            // we default to a decent sized upper bound instead
            assert_eq!(
                UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            // as we add mixnodes, we're always inserting them in ascending manner due to monotonically increasing id
            let mut deps = test_helpers::init_contract();
            let identity = "foomp123";

            add_unbonded_with_identity(deps.as_mut().storage, 1, identity);

            let per_page = 2;
            let page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                Some(per_page),
            )
            .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            // save another
            add_unbonded_with_identity(deps.as_mut().storage, 2, identity);

            // page1 should have 2 results on it
            let page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, page1.nodes.len());

            add_unbonded_with_identity(deps.as_mut().storage, 3, identity);

            // page1 still has the same 2 results
            let another_page1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                None,
                Some(per_page),
            )
            .unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            add_unbonded_with_identity(deps.as_mut().storage, 4, identity);
            let page2 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity.into(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }

        #[test]
        fn only_retrieves_nodes_with_specific_identity_key() {
            let mut deps = test_helpers::init_contract();
            let identity1 = "identity1";
            let identity2 = "identity2";
            let identity3 = "identity3";
            let identity4 = "identity4";

            add_unbonded_with_identity(deps.as_mut().storage, 1, identity1);
            add_unbonded_with_identity(deps.as_mut().storage, 2, identity1);
            add_unbonded_with_identity(deps.as_mut().storage, 3, identity2);
            add_unbonded_with_identity(deps.as_mut().storage, 4, identity1);
            add_unbonded_with_identity(deps.as_mut().storage, 5, identity3);
            add_unbonded_with_identity(deps.as_mut().storage, 6, identity3);
            add_unbonded_with_identity(deps.as_mut().storage, 7, identity4);
            add_unbonded_with_identity(deps.as_mut().storage, 8, identity2);
            add_unbonded_with_identity(deps.as_mut().storage, 9, identity1);
            add_unbonded_with_identity(deps.as_mut().storage, 10, identity3);

            let expected_ids1 = vec![1, 2, 4, 9];
            let expected_ids2 = vec![3, 8];
            let expected_ids3 = vec![5, 6, 10];
            let expected_ids4 = vec![7];

            let res1 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity1.into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert_eq!(res1, expected_ids1);

            let res2 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity2.into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert_eq!(res2, expected_ids2);

            let res3 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity3.into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert_eq!(res3, expected_ids3);

            let res4 = query_unbonded_mixnodes_by_identity_paged(
                deps.as_ref(),
                identity4.into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert_eq!(res4, expected_ids4);

            let res5 = query_unbonded_mixnodes_by_owner_paged(
                deps.as_ref(),
                "doesnt-exist".into(),
                None,
                None,
            )
            .unwrap()
            .nodes
            .into_iter()
            .map(|r| r.0)
            .collect::<Vec<_>>();
            assert!(res5.is_empty());
        }
    }

    #[test]
    fn query_for_owned_mixnode() {
        let mut test = TestSetup::new();

        let address = "mix-owner".to_string();

        // when it doesnt exist
        let res = query_owned_mixnode(test.deps(), address.clone()).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(address, res.address);

        // when it [fully] exists
        let id = test.add_legacy_mixnode(&address, None);
        let res = query_owned_mixnode(test.deps(), address.clone()).unwrap();
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
            .save(test.deps_mut().storage, id, &rewarding_details)
            .unwrap();

        pending_events::unbond_mixnode(test.deps_mut(), &mock_env(), 123, id).unwrap();
        let res = query_owned_mixnode(test.deps(), address.clone()).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(address, res.address);
    }

    #[test]
    fn query_for_mixnode_details() {
        let mut test = TestSetup::new();

        // no node under this id
        let res = query_mixnode_details(test.deps(), 42).unwrap();
        assert!(res.mixnode_details.is_none());
        assert_eq!(42, res.mix_id);

        // it exists
        let mix_id = test.add_legacy_mixnode("foomp", None);
        let res = query_mixnode_details(test.deps(), mix_id).unwrap();
        let details = res.mixnode_details.unwrap();
        assert_eq!(mix_id, details.bond_information.mix_id);
        assert_eq!(
            good_mixnode_pledge()[0],
            details.bond_information.original_pledge
        );
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_mixnode_details_by_identity() {
        let mut test = TestSetup::new();

        // no node under this identity
        let res = query_mixnode_details_by_identity(test.deps(), "foomp".into())
            .unwrap()
            .mixnode_details;
        assert!(res.is_none());

        // it exists
        let mix_id = test.add_legacy_mixnode("owner", None);
        // this was already tested to be working : )
        let expected = query_mixnode_details(test.deps(), mix_id)
            .unwrap()
            .mixnode_details
            .unwrap();
        let mix_identity = expected.bond_information.identity();

        let res = query_mixnode_details_by_identity(test.deps(), mix_identity.into())
            .unwrap()
            .mixnode_details;
        assert_eq!(expected, res.unwrap());
    }

    #[test]
    fn query_for_mixnode_rewarding_details() {
        let mut test = TestSetup::new();

        // no node under this id
        let res = query_mixnode_rewarding_details(test.deps(), 42).unwrap();
        assert!(res.rewarding_details.is_none());
        assert_eq!(42, res.mix_id);

        let mix_id = test.add_legacy_mixnode("foomp", None);
        let res = query_mixnode_rewarding_details(test.deps(), mix_id).unwrap();
        let details = res.rewarding_details.unwrap();
        assert_eq!(fixtures::node_cost_params_fixture(), details.cost_params);
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_unbonded_mixnode() {
        let mut test = TestSetup::new();

        let sender = "mix-owner";

        // no node under this id
        let res = query_unbonded_mixnode(test.deps(), 42).unwrap();
        assert!(res.unbonded_info.is_none());
        assert_eq!(42, res.mix_id);

        // add and unbond the mixnode
        let mix_id = test.add_legacy_mixnode(sender, None);
        pending_events::unbond_mixnode(test.deps_mut(), &mock_env(), 123, mix_id).unwrap();

        let res = query_unbonded_mixnode(test.deps(), mix_id).unwrap();
        assert_eq!(res.unbonded_info.unwrap().owner, sender);
        assert_eq!(mix_id, res.mix_id);
    }

    #[test]
    fn query_for_stake_saturation() {
        let mut test = TestSetup::new();

        // no node under this id
        let res = query_stake_saturation(test.deps(), 42).unwrap();
        assert!(res.current_saturation.is_none());
        assert!(res.uncapped_saturation.is_none());
        assert_eq!(42, res.mix_id);

        let rewarding_params = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();
        let saturation_point = rewarding_params.interval.stake_saturation_point;

        let mix_id = test.add_legacy_mixnode("foomp", None);

        // below saturation point
        // there's only the base pledge without any delegation
        let expected =
            Decimal::from_atomics(good_mixnode_pledge()[0].amount, 0).unwrap() / saturation_point;
        let res = query_stake_saturation(test.deps(), mix_id).unwrap();
        assert_eq!(expected, res.current_saturation.unwrap());
        assert_eq!(expected, res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);

        // exactly at saturation point
        let mut mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .load(test.deps().storage, mix_id)
            .unwrap();
        mix_rewarding.operator = saturation_point;
        rewards_storage::MIXNODE_REWARDING
            .save(test.deps_mut().storage, mix_id, &mix_rewarding)
            .unwrap();

        let res = query_stake_saturation(test.deps(), mix_id).unwrap();
        assert_eq!(Decimal::one(), res.current_saturation.unwrap());
        assert_eq!(Decimal::one(), res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);

        // above the saturation point
        let mut mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .load(test.deps().storage, mix_id)
            .unwrap();
        mix_rewarding.delegates = mix_rewarding.operator * Decimal::percent(150);
        rewards_storage::MIXNODE_REWARDING
            .save(test.deps_mut().storage, mix_id, &mix_rewarding)
            .unwrap();

        let res = query_stake_saturation(test.deps(), mix_id).unwrap();
        assert_eq!(Decimal::one(), res.current_saturation.unwrap());
        assert_eq!(Decimal::percent(250), res.uncapped_saturation.unwrap());
        assert_eq!(mix_id, res.mix_id);
    }
}
