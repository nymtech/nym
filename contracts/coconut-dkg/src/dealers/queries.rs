// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{
    self, get_dealer_details, get_dealer_index, get_registration_details, DEALERS_INDICES,
    EPOCH_DEALERS_MAP,
};
use crate::epoch_state::storage::CURRENT_EPOCH;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_coconut_dkg_common::dealer::{
    DealerDetailsResponse, DealerType, PagedDealerIndexResponse, PagedDealerResponse,
    RegisteredDealerDetails,
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
        None => CURRENT_EPOCH.load(deps.storage)?.epoch_id,
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
    let current_epoch_id = CURRENT_EPOCH.load(deps.storage)?.epoch_id;

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

pub fn query_current_dealers_paged(
    deps: Deps<'_>,
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

    let current_epoch_id = CURRENT_EPOCH.load(deps.storage)?.epoch_id;

    let dealers = EPOCH_DEALERS_MAP
        .prefix(current_epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(address, details)| {
                // SAFETY: if we have DealerRegistrationDetails saved, it means we MUST also have its node index
                // otherwise some serious invariants have been broken in the contract, and we're in trouble
                #[allow(clippy::expect_used)]
                let assigned_index = get_dealer_index(deps.storage, &address, current_epoch_id)
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dealers::storage::{DEALERS_PAGE_DEFAULT_LIMIT, DEALERS_PAGE_MAX_LIMIT};
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers::{init_contract, insert_dealer};
    use cosmwasm_std::DepsMut;

    fn fill_dealers(mut deps: DepsMut<'_>, epoch_id: EpochId, size: usize) {
        for assigned_index in 0..size {
            let dealer_details = dealer_details_fixture(assigned_index as u64);
            insert_dealer(deps.branch(), epoch_id, &dealer_details);
        }
    }

    fn remove_dealers(deps: DepsMut<'_>, epoch_id: EpochId, size: usize) {
        for assigned_index in 0..size {
            let dealer_details = dealer_details_fixture(assigned_index as u64);
            DEALERS_INDICES.remove(deps.storage, &dealer_details.address);

            EPOCH_DEALERS_MAP.remove(deps.storage, (epoch_id, &dealer_details.address));
        }
    }

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

        fill_dealers(deps.as_mut(), 0, 1000);

        let page1 = query_current_dealers_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.dealers.len() as u32);

        remove_dealers(deps.as_mut(), 0, 1000);
    }

    #[test]
    fn dealers_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();

        fill_dealers(deps.as_mut(), 0, 1000);

        // query without explicitly setting a limit
        let page1 = query_current_dealers_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(DEALERS_PAGE_DEFAULT_LIMIT, page1.dealers.len() as u32);

        remove_dealers(deps.as_mut(), 0, 1000);
    }

    #[test]
    fn dealers_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * DEALERS_PAGE_MAX_LIMIT;

        fill_dealers(deps.as_mut(), 0, 1000);

        let page1 =
            query_current_dealers_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = DEALERS_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.dealers.len() as u32);

        remove_dealers(deps.as_mut(), 0, 1000);
    }

    #[test]
    fn dealers_pagination_works() {
        let mut deps = init_contract();

        let per_page = 2;

        fill_dealers(deps.as_mut(), 0, 1);
        let page1 =
            query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.dealers.len());
        remove_dealers(deps.as_mut(), 0, 1);

        fill_dealers(deps.as_mut(), 0, 2);
        // page1 should have 2 results on it
        let page1 =
            query_current_dealers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.dealers.len());
        remove_dealers(deps.as_mut(), 0, 2);

        fill_dealers(deps.as_mut(), 0, 3);
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
        remove_dealers(deps.as_mut(), 0, 3);

        fill_dealers(deps.as_mut(), 0, 4);
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
        remove_dealers(deps.as_mut(), 0, 4);
    }
}
