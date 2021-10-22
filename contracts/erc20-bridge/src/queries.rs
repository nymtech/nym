// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Deps, Order, StdResult};

use crate::storage::payments_read;
use erc20_bridge_contract::keys::PublicKey;
use erc20_bridge_contract::payment::{PagedPaymentResponse, Payment};

pub fn query_payments_paged(
    deps: Deps,
    _start_after: Option<PublicKey>,
    limit: Option<u32>,
) -> StdResult<PagedPaymentResponse> {
    let limit = limit.unwrap_or(0).min(0) as usize;
    let start: Option<Vec<u8>> = None;

    let payments = payments_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1.clone()))
        .collect::<StdResult<Vec<Payment>>>()?;

    let start_next_after = payments.last().map(|payment| payment.verification_key());

    Ok(PagedPaymentResponse::new(payments, limit, start_next_after))
}
