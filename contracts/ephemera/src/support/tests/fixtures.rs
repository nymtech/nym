// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use nym_ephemera_common::types::JsonPeerInfo;

pub const TEST_MIX_DENOM: &str = "unym";

pub fn peer_fixture(cosmos_address: &str) -> JsonPeerInfo {
    JsonPeerInfo {
        cosmos_address: Addr::unchecked(cosmos_address),
        ip_address: "127.0.0.1".to_string(),
        public_key: "random_key".to_string(),
    }
}
