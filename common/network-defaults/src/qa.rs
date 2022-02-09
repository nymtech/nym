// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "nymt1fp8wa64e2aukznmrs8n9v743uurh5usnvapw09";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "nymt1xwqrvxj8efs93a6l7rzu43tx7e53924hrpfatk";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "nymt1dn52nx8wv9wkqmrvj6tcmdzh4es6jt8tr7f6j9";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.nymtech.net",
        Some("https://qa-validator.nymtech.net/api"),
    )]
}
