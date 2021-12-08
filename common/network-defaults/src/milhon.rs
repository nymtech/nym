// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const BECH32_PREFIX: &str = "punk";
pub const DENOM: &str = "upunk";

pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";
pub const DEFAULT_VESTING_CONTRACT_ADDRESS: &str = "";
pub const COSMOS_CONTRACT_ADDRESS: &str = "punk1jld76tqw4wnpfenmay2xkv86nr3j0w426eka82";
pub const REWARDING_VALIDATOR_ADDRESS: &str = "punk1v9qauwdq5terag6uvfsdytcs2d0sdmfdy7hgk3";
pub const ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("9fEE3e28c17dbB87310A51F13C4fbf4331A6f102");

// Name of the event triggered by the eth contract. If the event name is changed,
// this would also need to be changed; It is currently tested against the json abi
pub const ETH_EVENT_NAME: &str = "Burned";
pub const ETH_BURN_FUNCTION_NAME: &str = "burnTokenForAccessCode";
