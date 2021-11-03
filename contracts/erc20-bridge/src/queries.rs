// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Deps, Order, StdResult};

use crate::storage::payments_read;
use erc20_bridge_contract::keys::PublicKey;
use erc20_bridge_contract::payment::{PagedPaymentResponse, Payment};

const BOND_PAGE_MAX_LIMIT: u32 = 100;
const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

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
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let payments = payments_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<Payment>>>()?;

    let start_next_after = payments.last().map(|payment| payment.verification_key());

    Ok(PagedPaymentResponse::new(payments, limit, start_next_after))
}
