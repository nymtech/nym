// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::var_names;
use crate::{DenomDetails, ValidatorDetails};

pub(crate) const BECH32_PREFIX: &str = "n";

pub const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

pub const MIXNET_CONTRACT_ADDRESS: &str =
    "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr";
pub const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("0000000000000000000000000000000000000000");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy";

pub const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "https://mainnet-stats.nymte.ch:8090/";
pub const NYXD_URL: &str = "https://rpc.nymtech.net";
pub const NYM_API: &str = "https://validator.nymtech.net/api/";
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(NYXD_URL, Some(NYM_API))]
}

const DEFAULT_SUFFIX: &str = "_MAINNET_DEFAULT";

fn set_var_to_default(var: &str, value: &str) {
    std::env::set_var(var, value);
    std::env::set_var(format!("{var}{DEFAULT_SUFFIX}"), "1")
}

fn set_var_conditionally_to_default(var: &str, value: &str) {
    if std::env::var(var).is_err() {
        set_var_to_default(var, value)
    }
}

pub fn uses_default(var: &str) -> bool {
    std::env::var(format!("{var}{DEFAULT_SUFFIX}")).is_ok()
}

pub fn read_var_if_not_default(var: &str) -> Option<String> {
    if uses_default(var) {
        None
    } else {
        std::env::var(var).ok()
    }
}

pub fn export_to_env() {
    set_var_to_default(var_names::CONFIGURED, "true");
    set_var_to_default(var_names::BECH32_PREFIX, BECH32_PREFIX);
    set_var_to_default(var_names::MIX_DENOM, MIX_DENOM.base);
    set_var_to_default(var_names::MIX_DENOM_DISPLAY, MIX_DENOM.display);
    set_var_to_default(var_names::STAKE_DENOM, STAKE_DENOM.base);
    set_var_to_default(var_names::STAKE_DENOM_DISPLAY, STAKE_DENOM.display);
    set_var_to_default(
        var_names::DENOMS_EXPONENT,
        &STAKE_DENOM.display_exponent.to_string(),
    );
    set_var_to_default(var_names::MIXNET_CONTRACT_ADDRESS, MIXNET_CONTRACT_ADDRESS);
    set_var_to_default(
        var_names::VESTING_CONTRACT_ADDRESS,
        VESTING_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
        BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
        COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
    );
    set_var_to_default(var_names::GROUP_CONTRACT_ADDRESS, GROUP_CONTRACT_ADDRESS);
    set_var_to_default(
        var_names::MULTISIG_CONTRACT_ADDRESS,
        MULTISIG_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::COCONUT_DKG_CONTRACT_ADDRESS,
        COCONUT_DKG_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::REWARDING_VALIDATOR_ADDRESS,
        REWARDING_VALIDATOR_ADDRESS,
    );
    set_var_to_default(
        var_names::STATISTICS_SERVICE_DOMAIN_ADDRESS,
        STATISTICS_SERVICE_DOMAIN_ADDRESS,
    );
    set_var_to_default(var_names::NYXD, NYXD_URL);
    set_var_to_default(var_names::NYM_API, NYM_API);
}

pub fn export_to_env_if_not_set() {
    set_var_conditionally_to_default(var_names::CONFIGURED, "true");
    set_var_conditionally_to_default(var_names::BECH32_PREFIX, BECH32_PREFIX);
    set_var_conditionally_to_default(var_names::MIX_DENOM, MIX_DENOM.base);
    set_var_conditionally_to_default(var_names::MIX_DENOM_DISPLAY, MIX_DENOM.display);
    set_var_conditionally_to_default(var_names::STAKE_DENOM, STAKE_DENOM.base);
    set_var_conditionally_to_default(var_names::STAKE_DENOM_DISPLAY, STAKE_DENOM.display);
    set_var_conditionally_to_default(
        var_names::DENOMS_EXPONENT,
        &STAKE_DENOM.display_exponent.to_string(),
    );
    set_var_conditionally_to_default(var_names::MIXNET_CONTRACT_ADDRESS, MIXNET_CONTRACT_ADDRESS);
    set_var_conditionally_to_default(
        var_names::VESTING_CONTRACT_ADDRESS,
        VESTING_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
        BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
        COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(var_names::GROUP_CONTRACT_ADDRESS, GROUP_CONTRACT_ADDRESS);
    set_var_conditionally_to_default(
        var_names::MULTISIG_CONTRACT_ADDRESS,
        MULTISIG_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::COCONUT_DKG_CONTRACT_ADDRESS,
        COCONUT_DKG_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::REWARDING_VALIDATOR_ADDRESS,
        REWARDING_VALIDATOR_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::STATISTICS_SERVICE_DOMAIN_ADDRESS,
        STATISTICS_SERVICE_DOMAIN_ADDRESS,
    );
    set_var_conditionally_to_default(var_names::NYXD, NYXD_URL);
    set_var_conditionally_to_default(var_names::NYM_API, NYM_API);
}
