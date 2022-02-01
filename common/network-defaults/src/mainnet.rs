// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const DENOM: &str = "unym";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://rpc.nyx.nodes.guru/",
        Some("https://api.nyx.nodes.guru/"),
    )]
}
