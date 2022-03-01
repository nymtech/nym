// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::storage;
use coconut_dkg_common::types::{
    EncodedChannelPublicKey, InactiveIssuerDetailsResponse, IssuerDetailsResponse, NodeIndex,
};
use cosmwasm_std::Deps;

pub(crate) fn query_current_issuer_details(
    deps: Deps<'_>,
    address: &str,
) -> Result<IssuerDetailsResponse, ContractError> {
    let validated = deps.api.addr_validate(address)?;
    let details = storage::CURRENT_ISSUERS.may_load(deps.storage, validated)?;

    Ok(IssuerDetailsResponse { details })
}

pub(crate) fn query_inactive_issuer_details(
    deps: Deps<'_>,
    address: &str,
) -> Result<IssuerDetailsResponse, ContractError> {
    let validated = deps.api.addr_validate(address)?;
    let (details_and_last_seen) = storage::INACTIVE_ISSUERS.may_load(deps.storage, validated)?;

    todo!()
    //     .unzip();
    //
    // Ok(InactiveIssuerDetailsResponse { details, last_seen })
}
