// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage;
use crate::dealings::storage::DEALING_COMMITMENTS;
use coconut_dkg_common::dealer::{ContractDealingCommitment, PagedCommitmentsResponse};
use coconut_dkg_common::types::EpochId;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

pub fn query_epoch_dealings_commitments_paged(
    deps: Deps<'_>,
    epoch: EpochId,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedCommitmentsResponse> {
    let limit = limit
        .unwrap_or(storage::COMMITMENTS_PAGE_DEFAULT_LIMIT)
        .min(storage::COMMITMENTS_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let commitments = DEALING_COMMITMENTS
        .prefix(epoch)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(dealer, commitment)| {
                ContractDealingCommitment::new(commitment, dealer, epoch)
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = commitments
        .last()
        .map(|commitment| commitment.dealer.clone());

    Ok(PagedCommitmentsResponse::new(
        commitments,
        limit,
        start_next_after,
    ))
}
