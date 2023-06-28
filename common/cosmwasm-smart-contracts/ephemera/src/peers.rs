// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::JsonPeerInfo;
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedPeerResponse {
    pub peers: Vec<JsonPeerInfo>,
    pub per_page: usize,
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
