// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";

pub const MIXNET_CONTRACT_ADDRESS: &str = "nymt14hj2tavq8fpesdwxxcu44rty3hh90vhuysqrsr";
pub const VESTING_CONTRACT_ADDRESS: &str = "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub const REWARDING_VALIDATOR_ADDRESS: &str = "nymt1dn52nx8wv9wkqmrvj6tcmdzh4es6jt8tr7f6j9";

pub fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.nymtech.net",
        Some("https://qa-validator.nymtech.net/api"),
    )]
}
