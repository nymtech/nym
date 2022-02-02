// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "n";
pub const DENOM: &str = "unym";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n17zujduc46wvkwvp6f062mm5xhr7jc3fewvqu9e";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://rpc.nyx.nodes.guru/",
        Some("https://api.nyx.nodes.guru/"),
    )]
}
