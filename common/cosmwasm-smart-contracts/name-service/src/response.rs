// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{NameId, RegisteredName};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct NamesListResponse {
    pub names: Vec<RegisteredName>,
}

impl NamesListResponse {
    pub fn new(names: Vec<RegisteredName>) -> NamesListResponse {
        NamesListResponse { names }
    }
}

impl From<&[RegisteredName]> for NamesListResponse {
    fn from(names: &[RegisteredName]) -> Self {
        NamesListResponse {
            names: names.to_vec(),
        }
    }
}

#[cw_serde]
pub struct PagedNamesListResponse {
    pub names: Vec<RegisteredName>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<NameId>,
}

impl PagedNamesListResponse {
    pub fn new(
        names: Vec<RegisteredName>,
        per_page: usize,
        start_next_after: Option<NameId>,
    ) -> PagedNamesListResponse {
        PagedNamesListResponse {
            names,
            per_page,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct ConfigResponse {
    pub deposit_required: Coin,
}
