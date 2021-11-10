// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Deps, Order, StdResult};

use crate::storage::payments_read;
use erc20_bridge_contract::keys::PublicKey;
use erc20_bridge_contract::payment::{PagedPaymentResponse, Payment};

const PAYMENT_PAGE_MAX_LIMIT: u32 = 100;
const PAYMENT_PAGE_DEFAULT_LIMIT: u32 = 50;

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
fn calculate_start_value<B: AsRef<[u8]>>(start_after: Option<B>) -> Option<Vec<u8>> {
    start_after.as_ref().map(|identity| {
        identity
            .as_ref()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect()
    })
}

pub fn query_payments_paged(
    deps: Deps,
    start_after: Option<PublicKey>,
    limit: Option<u32>,
) -> StdResult<PagedPaymentResponse> {
    let limit = limit
        .unwrap_or(PAYMENT_PAGE_DEFAULT_LIMIT)
        .min(PAYMENT_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let payments = payments_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<Payment>>>()?;

    let start_next_after = payments.last().map(|payment| payment.verification_key());

    Ok(PagedPaymentResponse::new(payments, limit, start_next_after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::payments;
    use crate::support::tests::helpers;
    use std::convert::TryInto;

    #[test]
    fn payments_empty_on_init() {
        let deps = helpers::init_contract();
        let response = query_payments_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.payments.len());
    }

    #[test]
    fn payments_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        for n in 0u32..10000 {
            let bytes: Vec<u8> = std::iter::repeat(n.to_be_bytes())
                .take(8)
                .flatten()
                .collect();
            let verification_key = PublicKey::new(bytes.try_into().unwrap());
            let payment = helpers::payment_fixture();
            payments(storage)
                .save(&verification_key.to_bytes(), &payment)
                .unwrap();
        }

        let page1 = query_payments_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.payments.len() as u32);
    }

    #[test]
    fn payments_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0u32..100 {
            let bytes: Vec<u8> = std::iter::repeat(n.to_be_bytes())
                .take(8)
                .flatten()
                .collect();
            let verification_key = PublicKey::new(bytes.try_into().unwrap());
            let payment = helpers::payment_fixture();
            payments(storage)
                .save(&verification_key.to_bytes(), &payment)
                .unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_payments_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(PAYMENT_PAGE_DEFAULT_LIMIT, page1.payments.len() as u32);
    }

    #[test]
    fn payments_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0u32..10000 {
            let bytes: Vec<u8> = std::iter::repeat(n.to_be_bytes())
                .take(8)
                .flatten()
                .collect();
            let verification_key = PublicKey::new(bytes.try_into().unwrap());
            let payment = helpers::payment_fixture();
            payments(storage)
                .save(&verification_key.to_bytes(), &payment)
                .unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000;
        let page1 = query_payments_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        assert_eq!(PAYMENT_PAGE_MAX_LIMIT, page1.payments.len() as u32);
    }

    #[test]
    fn payments_pagination_works() {
        let key1 = PublicKey::new([1; 32]);
        let key2 = PublicKey::new([2; 32]);
        let key3 = PublicKey::new([3; 32]);
        let key4 = PublicKey::new([4; 32]);

        let mut deps = helpers::init_contract();
        let payment = helpers::payment_fixture();
        payments(&mut deps.storage)
            .save(&key1.to_bytes(), &payment)
            .unwrap();

        let per_page = 2;
        let page1 = query_payments_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.payments.len());

        // save another
        payments(&mut deps.storage)
            .save(&key2.to_bytes(), &payment)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_payments_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.payments.len());

        payments(&mut deps.storage)
            .save(&key3.to_bytes(), &payment)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_payments_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.payments.len());

        // retrieving the next page should start after the last key on this page
        let start_after = key2;
        let page2 = query_payments_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.payments.len());

        // save another one
        payments(&mut deps.storage)
            .save(&key4.to_bytes(), &payment)
            .unwrap();

        let start_after = key2;
        let page2 = query_payments_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.payments.len());
    }
}
