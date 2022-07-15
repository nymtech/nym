// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::var_names;
use crate::{DenomDetails, ValidatorDetails};

pub(crate) const BECH32_PREFIX: &str = "n";

pub const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy";

pub(crate) const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "http://127.0.0.1:8090";
pub const NYMD_VALIDATOR: &str = "https://rpc.nyx.nodes.guru/";
pub const API_VALIDATOR: &str = "https://validator.nymtech.net/api/";
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(NYMD_VALIDATOR, Some(API_VALIDATOR))]
}

pub fn export_to_env() {
    std::env::set_var(var_names::BECH32_PREFIX, BECH32_PREFIX);
    std::env::set_var(var_names::MIX_DENOM, MIX_DENOM.base);
    std::env::set_var(var_names::MIX_DENOM_DISPLAY, MIX_DENOM.display);
    std::env::set_var(var_names::STAKE_DENOM, STAKE_DENOM.base);
    std::env::set_var(var_names::STAKE_DENOM_DISPLAY, STAKE_DENOM.display);
    std::env::set_var(
        var_names::DENOMS_EXPONENT,
        STAKE_DENOM.display_exponent.to_string(),
    );
    std::env::set_var(var_names::MIXNET_CONTRACT_ADDRESS, MIXNET_CONTRACT_ADDRESS);
    std::env::set_var(
        var_names::VESTING_CONTRACT_ADDRESS,
        VESTING_CONTRACT_ADDRESS,
    );
    std::env::set_var(
        var_names::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
        BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    );
    std::env::set_var(
        var_names::COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
        COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
    );
    std::env::set_var(
        var_names::MULTISIG_CONTRACT_ADDRESS,
        MULTISIG_CONTRACT_ADDRESS,
    );
    std::env::set_var(
        var_names::REWARDING_VALIDATOR_ADDRESS,
        REWARDING_VALIDATOR_ADDRESS,
    );
    std::env::set_var(
        var_names::STATISTICS_SERVICE_DOMAIN_ADDRESS,
        STATISTICS_SERVICE_DOMAIN_ADDRESS,
    );
    std::env::set_var(var_names::NYMD_VALIDATOR, NYMD_VALIDATOR);
    std::env::set_var(var_names::API_VALIDATOR, API_VALIDATOR);
}
