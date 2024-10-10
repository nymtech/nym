// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::constants::{GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT, GATEWAY_BOND_MAX_RETRIEVAL_LIMIT}; // Keeps gateway and mixnode retrieval in sync by re-using the constant. Could be split into its own constant.
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use mixnet_contract_common::gateway::{PreassignedGatewayIdsResponse, PreassignedId};
use mixnet_contract_common::{
    GatewayBond, GatewayBondResponse, GatewayOwnershipResponse, IdentityKey, PagedGatewayResponse,
};

pub(crate) fn query_gateways_paged(
    deps: Deps<'_>,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit
        .unwrap_or(GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT)
        .min(GATEWAY_BOND_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.as_deref().map(Bound::exclusive);

    let nodes = storage::gateways()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedGatewayResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_owned_gateway(
    deps: Deps<'_>,
    address: String,
) -> StdResult<GatewayOwnershipResponse> {
    let validated_addr = deps.api.addr_validate(&address)?;

    let gateway = storage::gateways()
        .idx
        .owner
        .item(deps.storage, validated_addr.clone())?
        .map(|record| record.1);

    Ok(GatewayOwnershipResponse {
        address: validated_addr,
        gateway,
    })
}

pub fn query_gateway_bond(deps: Deps<'_>, identity: IdentityKey) -> StdResult<GatewayBondResponse> {
    Ok(GatewayBondResponse {
        gateway: storage::gateways().may_load(deps.storage, &identity)?,
        identity,
    })
}

pub(crate) fn query_preassigned_ids_paged(
    deps: Deps<'_>,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PreassignedGatewayIdsResponse> {
    let limit = limit.unwrap_or(50).min(100) as usize;

    let start = start_after.as_deref().map(Bound::exclusive);

    let ids = storage::PREASSIGNED_LEGACY_IDS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(identity, node_id)| PreassignedId { identity, node_id }))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = ids.last().map(|id| id.identity.clone());

    Ok(PreassignedGatewayIdsResponse {
        ids,
        start_next_after,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::testing::mock_info;

    #[test]
    fn gateways_empty_on_init() {
        let deps = test_helpers::init_contract();
        let response = query_gateways_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn gateways_paged_retrieval_obeys_limits() {
        let mut test = TestSetup::new();
        test.add_dummy_gateways(1000);

        let limit = 2;

        let page1 = query_gateways_paged(test.deps(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_default_limit() {
        let mut test = TestSetup::new();
        test.add_dummy_gateways(1000);

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(test.deps(), None, None).unwrap();

        assert_eq!(
            GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT,
            page1.nodes.len() as u32
        );
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut test = TestSetup::new();
        test.add_dummy_gateways(1000);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT;
        let page1 = query_gateways_paged(test.deps(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = GATEWAY_BOND_MAX_RETRIEVAL_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        let stake = tests::fixtures::good_gateway_pledge();
        let mut test = TestSetup::new();

        // prepare 4 gateways that are sorted by the generated identities
        // (because we query them in an ascended manner)
        let mut gateways = (0..4)
            .map(|i| test.gateway_with_signature(format!("sender{}", i), None).0)
            .collect::<Vec<_>>();
        gateways.sort_by(|g1, g2| g1.identity_key.cmp(&g2.identity_key));

        let info = mock_info("sender0", &stake);
        test.save_legacy_gateway(gateways[0].clone(), &info);

        let per_page = 2;
        let page1 = query_gateways_paged(test.deps(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        let info = mock_info("sender1", &stake);
        test.save_legacy_gateway(gateways[1].clone(), &info);

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(test.deps(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        let info = mock_info("sender2", &stake);
        test.save_legacy_gateway(gateways[2].clone(), &info);

        // page1 still has 2 results
        let page1 = query_gateways_paged(test.deps(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_gateways_paged(
            test.deps(),
            Option::from(start_after.clone()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        let info = mock_info("sender3", &stake);
        test.save_legacy_gateway(gateways[3].clone(), &info);

        let page2 = query_gateways_paged(
            test.deps(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.nodes.len());
    }

    #[test]
    fn query_for_gateway_owner_works() {
        let mut test = TestSetup::new();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owned_gateway(test.deps(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());

        // gateway was added to "bob", "fred" still does not own one
        test.add_legacy_gateway("bob", None);

        let res = query_owned_gateway(test.deps(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());

        // "fred" now owns a gateway!
        test.add_legacy_gateway("fred", None);

        let res = query_owned_gateway(test.deps(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_some());

        // but after unbonding it, he doesn't own one anymore
        crate::gateways::transactions::try_remove_gateway(test.deps_mut(), mock_info("fred", &[]))
            .unwrap();

        let res = query_owned_gateway(test.deps(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());
    }
}
