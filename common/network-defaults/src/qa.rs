// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "n";
pub const DENOM: &str = "unym";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsd3qaep";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n1tfzd4qz3a45u8p4mr5zmzv66457uwjgcl05jdq";

pub(crate) const STATS_PROVIDER_CLIENT_ADDRESS: &str = "BLFPkyQ68xtR3TmrUWJZUKJF4SVwJR23wzQEmLHi2QcZ.5zms2X4ANsgY1VB4iC9kTqvbsHWmWUNSuvTtYr4Cp5qT@ExyJVqTSrgHTwzXm2r9RawfF5qYpvZjSVN2dLTs6bnWH";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.nymtech.net",
        Some("https://qa-validator-api.nymtech.net/api"),
    )]
}
