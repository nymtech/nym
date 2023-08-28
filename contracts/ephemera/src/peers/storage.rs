// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw_storage_plus::Map;
use nym_ephemera_common::types::JsonPeerInfo;

pub(crate) const PEERS_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const PEERS_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) const PEERS: Map<'_, Addr, JsonPeerInfo> = Map::new("prs");
