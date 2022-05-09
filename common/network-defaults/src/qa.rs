// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "nymt17x6pt4msccvawgxjeg5nmnygttu56tftg5l6j3";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "nymt1t4dmskxea0avvrj8xtmu66hv7dkyg9s8059t3c";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "nymt1ghd753shjuwexxywmgs4xz7x2q732vcnstz02j";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "nymt1dn52nx8wv9wkqmrvj6tcmdzh4es6jt8tr7f6j9";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.nymtech.net",
        Some("https://qa-validator.nymtech.net/api"),
    )]
}
