// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{
    self, get_dealer_details, get_dealer_index, get_registration_details, DEALERS_INDICES,
    EPOCH_DEALERS_MAP,
};
use crate::epoch_state::storage::load_current_epoch;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_coconut_dkg_common::dealer::{
    DealerDetailsResponse, DealerType, PagedDealerAddressesResponse, PagedDealerIndexResponse,
    PagedDealerResponse, RegisteredDealerDetails,
};
use nym_coconut_dkg_common::types::{DealerDetails, EpochId};

pub fn query_registered_dealer_details(
    deps: Deps<'_>,
    dealer_address: String,
    epoch_id: Option<EpochId>,
) -> StdResult<RegisteredDealerDetails> {
    let addr = deps.api.addr_validate(&dealer_address)?;

    let epoch_id = match epoch_id {
        Some(epoch_id) => epoch_id,
        None => load_current_epoch(deps.storage)?.epoch_id,
    };

    Ok(RegisteredDealerDetails {
        details: get_registration_details(deps.storage, &addr, epoch_id).ok(),
    })
}

pub fn query_dealer_details(
    deps: Deps<'_>,
    dealer_address: String,
) -> StdResult<DealerDetailsResponse> {
    let addr = deps.api.addr_validate(&dealer_address)?;
    let current_epoch_id = load_current_epoch(deps.storage)?.epoch_id;

    // if the address has registration data for the current epoch, it means it's an active dealer
    if let Ok(dealer_details) = get_dealer_details(deps.storage, &addr, current_epoch_id) {
        let assigned_index = dealer_details.assigned_index;
        return Ok(DealerDetailsResponse::new(
            Some(dealer_details),
            DealerType::Current { assigned_index },
        ));
    }

    // and if has had an assigned index it must have been a dealer at some point in the past
    if let Ok(assigned_index) = get_dealer_index(deps.storage, &addr, current_epoch_id) {
        return Ok(DealerDetailsResponse::new(
            None,
            DealerType::Past { assigned_index },
        ));
    }

    Ok(DealerDetailsResponse::new(None, DealerType::Unknown))
}

pub fn query_dealers_indices_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerIndexResponse> {
    let limit = limit
        .unwrap_or(storage::DEALER_INDICES_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALER_INDICES_PAGE_MAX_LIMIT) as usize;
    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealers = DEALERS_INDICES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = dealers.last().map(|dealer| dealer.0.clone());

    Ok(PagedDealerIndexResponse::new(dealers, start_next_after))
}

pub fn query_epoch_dealers_addresses_paged(
    deps: Deps<'_>,
    epoch_id: EpochId,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerAddressesResponse> {
    let limit = limit
        .unwrap_or(storage::DEALERS_ADDRESSES_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALERS_ADDRESSES_PAGE_MAX_LIMIT) as usize;
    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealers = EPOCH_DEALERS_MAP
        .prefix(epoch_id)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;
    let start_next_after = dealers.last().cloned();

    Ok(PagedDealerAddressesResponse {
        dealers,
        start_next_after,
    })
}

pub fn query_epoch_dealers_paged(
    deps: Deps<'_>,
    epoch_id: EpochId,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    let limit = limit
        .unwrap_or(storage::DEALERS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALERS_PAGE_MAX_LIMIT) as usize;
    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealers = EPOCH_DEALERS_MAP
        .prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(address, details)| {
                // SAFETY: if we have DealerRegistrationDetails saved, it means we MUST also have its node index
                // otherwise some serious invariants have been broken in the contract, and we're in trouble
                #[allow(clippy::expect_used)]
                let assigned_index = get_dealer_index(deps.storage, &address, epoch_id)
                    .expect("could not retrieve dealer index for a registered dealer");

                DealerDetails {
                    address,
                    bte_public_key_with_proof: details.bte_public_key_with_proof,
                    ed25519_identity: details.ed25519_identity,
                    announce_address: details.announce_address,
                    assigned_index,
                }
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    let start_next_after = dealers.last().map(|dealer| dealer.address.clone());

    Ok(PagedDealerResponse::new(dealers, limit, start_next_after))
}

pub fn query_current_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    let current_epoch_id = load_current_epoch(deps.storage)?.epoch_id;
    query_epoch_dealers_paged(deps, current_epoch_id, start_after, limit)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dealers::storage::{DEALERS_PAGE_DEFAULT_LIMIT, DEALERS_PAGE_MAX_LIMIT};
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers::{init_contract, insert_dealer};
    use cosmwasm_std::testing::{MockApi, MockQuerier};
    use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

    fn fill_dealers(
        deps: &mut OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
        epoch_id: EpochId,
        size: usize,
    ) {
        for assigned_index in 0..size {
            let dealer_details = dealer_details_fixture(&deps.api, assigned_index as u64);
            insert_dealer(deps.as_mut(), epoch_id, &dealer_details);
        }
    }

    fn remove_dealers(
        deps: &mut OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
        epoch_id: EpochId,
        size: usize,
    ) {
        for assigned_index in 0..size {
            let dealer_details = dealer_details_fixture(&deps.api, assigned_index as u64);
            DEALERS_INDICES.remove(deps.as_mut().storage, &dealer_details.address);

            EPOCH_DEALERS_MAP.remove(deps.as_mut().storage, (epoch_id, &dealer_details.address));
        }
    }

    #[cfg(test)]
    mod current_epoch_dealers {
        use super::*;

        #[test]
        fn dealers_empty_on_init() {
            let deps = init_contract();

            let page1 = query_current_dealers_paged(deps.as_ref(), None, None).unwrap();
            assert_eq!(0, page1.dealers.len() as u32);
        }

        #[test]
        fn dealers_paged_retrieval_obeys_limits() {
            let mut deps = init_contract();
            let limit = 2;

            fill_dealers(&mut deps, 0, 1000);

            let page1 =
                query_current_dealers_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.dealers.len() as u32);

            remove_dealers(&mut deps, 0, 1000);
        }

        #[test]
        fn dealers_paged_retrieval_has_default_limit() {
            let mut deps = init_contract();

            fill_dealers(&mut deps, 0, 1000);

            // query without explicitly setting a limit
            let page1 = query_current_dealers_paged(deps.as_ref(), None, None).unwrap();

            assert_eq!(DEALERS_PAGE_DEFAULT_LIMIT, page1.dealers.len() as u32);

            remove_dealers(&mut deps, 0, 1000);
        }

        #[test]
        fn dealers_paged_retrieval_has_max_limit() {
            let mut deps = init_contract();

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * DEALERS_PAGE_MAX_LIMIT;

            fill_dealers(&mut deps, 0, 1000);

            let page1 = query_current_dealers_paged(deps.as_ref(), None, Option::from(crazy_limit))
                .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DEALERS_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.dealers.len() as u32);

            remove_dealers(&mut deps, 0, 1000);
        }

        #[test]
        fn dealers_pagination_works() {
            let mut deps = init_contract();

            let per_page = 2;

            fill_dealers(&mut deps, 0, 1);
            let page1 =
                query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.dealers.len());
            remove_dealers(&mut deps, 0, 1);

            fill_dealers(&mut deps, 0, 2);
            // page1 should have 2 results on it
            let page1 =
                query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealers.len());
            remove_dealers(&mut deps, 0, 2);

            fill_dealers(&mut deps, 0, 3);
            // page1 still has 2 results
            let page1 =
                query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealers.len());

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_current_dealers_paged(
                deps.as_ref(),
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.dealers.len());
            remove_dealers(&mut deps, 0, 3);

            fill_dealers(&mut deps, 0, 4);
            let page1 =
                query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_current_dealers_paged(
                deps.as_ref(),
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.dealers.len());
            remove_dealers(&mut deps, 0, 4);
        }
    }

    #[cfg(test)]
    mod epoch_dealers {
        use super::*;

        #[test]
        fn dealers_empty_on_init() {
            let deps = init_contract();

            // check few epochs
            for epoch_id in 0..10 {
                let page1 = query_epoch_dealers_paged(deps.as_ref(), epoch_id, None, None).unwrap();
                assert_eq!(0, page1.dealers.len() as u32);
            }
        }

        #[test]
        fn theres_no_ovewriting_between_epochs() {
            let mut deps = init_contract();

            fill_dealers(&mut deps, 1, 1000);

            let page1 = query_epoch_dealers_paged(deps.as_ref(), 1, None, None).unwrap();
            assert!(!page1.dealers.is_empty());

            // nothing for other epochs
            let another_epoch = query_epoch_dealers_paged(deps.as_ref(), 2, None, None).unwrap();
            assert!(another_epoch.dealers.is_empty());

            let another_epoch = query_epoch_dealers_paged(deps.as_ref(), 42, None, None).unwrap();
            assert!(another_epoch.dealers.is_empty());
        }

        #[test]
        fn dealers_paged_retrieval_obeys_limits() {
            let mut deps = init_contract();
            let limit = 2;

            fill_dealers(&mut deps, 0, 1000);

            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.dealers.len() as u32);
        }

        #[test]
        fn dealers_paged_retrieval_has_default_limit() {
            let mut deps = init_contract();

            fill_dealers(&mut deps, 0, 1000);

            // query without explicitly setting a limit
            let page1 = query_epoch_dealers_paged(deps.as_ref(), 0, None, None).unwrap();

            assert_eq!(DEALERS_PAGE_DEFAULT_LIMIT, page1.dealers.len() as u32);
        }

        #[test]
        fn dealers_paged_retrieval_has_max_limit() {
            let mut deps = init_contract();

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 1000 * DEALERS_PAGE_MAX_LIMIT;

            fill_dealers(&mut deps, 0, 1000);

            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(crazy_limit))
                    .unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DEALERS_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.dealers.len() as u32);
        }

        #[test]
        fn dealers_pagination_works() {
            let mut deps = init_contract();

            let per_page = 2;

            fill_dealers(&mut deps, 0, 1);
            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.dealers.len());
            remove_dealers(&mut deps, 0, 1);

            fill_dealers(&mut deps, 0, 2);
            // page1 should have 2 results on it
            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealers.len());
            remove_dealers(&mut deps, 0, 2);

            fill_dealers(&mut deps, 0, 3);
            // page1 still has 2 results
            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealers.len());

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_epoch_dealers_paged(
                deps.as_ref(),
                0,
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.dealers.len());
            remove_dealers(&mut deps, 0, 3);

            fill_dealers(&mut deps, 0, 4);
            let page1 =
                query_epoch_dealers_paged(deps.as_ref(), 0, None, Option::from(per_page)).unwrap();
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_epoch_dealers_paged(
                deps.as_ref(),
                0,
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.dealers.len());
        }
    }

    #[test]
    fn epoch_dealers_addresses() {
        let mut deps = init_contract();

        let mut fixtures = Vec::new();
        for i in 0..100 {
            let mut dealer_details = dealer_details_fixture(&deps.api, i);
            dealer_details.address = deps.api.addr_make(&format!("dummy-dealer-{i}"));
            fixtures.push(dealer_details);
        }

        // initially empty for all epochs
        for epoch_id in 0..10 {
            let page1 =
                query_epoch_dealers_addresses_paged(deps.as_ref(), epoch_id, None, None).unwrap();
            assert_eq!(0, page1.dealers.len() as u32);
        }

        // epoch0: dealers 0,1,2,3
        // epoch1: dealers 4,5,6
        // epoch2: dealers: 1,4,6 (some overlap)
        // epoch3: dealer 7
        // epoch4: dealers 0..100 (to check limits)
        insert_dealer(deps.as_mut(), 0, &fixtures[0]);
        insert_dealer(deps.as_mut(), 0, &fixtures[1]);
        insert_dealer(deps.as_mut(), 0, &fixtures[2]);
        insert_dealer(deps.as_mut(), 0, &fixtures[3]);

        insert_dealer(deps.as_mut(), 1, &fixtures[4]);
        insert_dealer(deps.as_mut(), 1, &fixtures[5]);
        insert_dealer(deps.as_mut(), 1, &fixtures[6]);

        insert_dealer(deps.as_mut(), 2, &fixtures[1]);
        insert_dealer(deps.as_mut(), 2, &fixtures[4]);
        insert_dealer(deps.as_mut(), 2, &fixtures[6]);

        insert_dealer(deps.as_mut(), 3, &fixtures[7]);

        for fixture in &fixtures {
            insert_dealer(deps.as_mut(), 4, fixture);
        }

        let res = query_epoch_dealers_addresses_paged(deps.as_ref(), 0, None, None).unwrap();
        assert_eq!(4, res.dealers.len() as u32);
        for fixture in &fixtures[0..=3] {
            assert!(res.dealers.contains(&fixture.address))
        }

        let res = query_epoch_dealers_addresses_paged(deps.as_ref(), 1, None, None).unwrap();
        assert_eq!(3, res.dealers.len() as u32);
        for fixture in &fixtures[4..=6] {
            assert!(res.dealers.contains(&fixture.address))
        }

        let res = query_epoch_dealers_addresses_paged(deps.as_ref(), 2, None, None).unwrap();
        assert_eq!(3, res.dealers.len() as u32);
        for fixture in &[
            fixtures[1].clone(),
            fixtures[4].clone(),
            fixtures[6].clone(),
        ] {
            assert!(res.dealers.contains(&fixture.address))
        }

        let res = query_epoch_dealers_addresses_paged(deps.as_ref(), 3, None, None).unwrap();
        assert_eq!(vec![fixtures[7].address.clone()], res.dealers);

        let res = query_epoch_dealers_addresses_paged(deps.as_ref(), 4, None, None).unwrap();
        assert_eq!(
            storage::DEALERS_ADDRESSES_PAGE_DEFAULT_LIMIT,
            res.dealers.len() as u32
        );

        let res =
            query_epoch_dealers_addresses_paged(deps.as_ref(), 4, None, Some(1000000)).unwrap();
        assert_eq!(
            storage::DEALERS_ADDRESSES_PAGE_MAX_LIMIT,
            res.dealers.len() as u32
        );
    }
}
