// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-milhon-validator1.nymtech.net:1317",
    "http://testnet-milhon-validator2.nymtech.net:1317",
];
pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";
pub const BECH32_PREFIX: &str = "punk";

pub const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;

// 'GATEWAY'
pub const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;

// 'MIXNODE'
pub const DEFAULT_VERLOC_LISTENING_PORT: u16 = 1790;
pub const DEFAULT_HTTP_API_LISTENING_PORT: u16 = 8000;
