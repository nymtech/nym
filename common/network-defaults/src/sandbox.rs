// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";

pub const MIXNET_CONTRACT_ADDRESS: &str = "nymt1ghd753shjuwexxywmgs4xz7x2q732vcnstz02j";
pub const VESTING_CONTRACT_ADDRESS: &str = "nymt1nc5tatafv6eyq7llkr2gv50ff9e22mnfp9pc5s";
pub const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub const REWARDING_VALIDATOR_ADDRESS: &str = "nymt17zujduc46wvkwvp6f062mm5xhr7jc3fewvqu9e";

pub fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://sandbox-validator.nymtech.net",
        Some("https://sandbox-validator.nymtech.net/api"),
    )]
}
