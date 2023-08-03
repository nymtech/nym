// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::JsonPeerInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct PagedPeerResponse {
    pub peers: Vec<JsonPeerInfo>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<Addr>,
}

impl PagedPeerResponse {
    pub fn new(peers: Vec<JsonPeerInfo>, per_page: usize, start_next_after: Option<Addr>) -> Self {
        PagedPeerResponse {
            peers,
            per_page,
            start_next_after,
        }
    }
}
