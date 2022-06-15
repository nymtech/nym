// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";
pub const STAKE_DENOM: &str = "unyxt";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "nymt1ghd753shjuwexxywmgs4xz7x2q732vcnstz02j";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "nymt14ejqjyq8um4p3xfqj74yld5waqljf88fn549lh";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "nymt1nz0r0au8aj6dc00wmm3ufy4g4k86rjzlgq608r";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("8e0DcFF7F3085235C32E845f3667aEB3f1e83133");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("E8883BAeF3869e14E4823F46662e81D4F7d2A81F");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "nymt1jh0s6qu6tuw9ut438836mmn7f3f2wencrnmdj4";

pub(crate) const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "";
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://sandbox-validator.nymtech.net",
        Some("https://sandbox-validator.nymtech.net/api"),
    )]
}
