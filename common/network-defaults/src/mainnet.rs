// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "n";
pub const DENOM: &str = "unym";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://rpc.nyx.nodes.guru/",
        Some("https://validator.nymtech.net/api"),
    )]
}
