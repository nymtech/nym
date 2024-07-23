// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "network")]
use crate::{DenomDetails, ValidatorDetails};

pub const NETWORK_NAME: &str = "mainnet";

pub const BECH32_PREFIX: &str = "n";

#[cfg(feature = "network")]
pub const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
#[cfg(feature = "network")]
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

pub const MIXNET_CONTRACT_ADDRESS: &str =
    "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr";
pub const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

pub const ECASH_CONTRACT_ADDRESS: &str = "";
pub const GROUP_CONTRACT_ADDRESS: &str =
    "n1e2zq4886zzewpvpucmlw8v9p7zv692f6yck4zjzxh699dkcmlrfqk2knsr";
pub const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1txayqfz5g9qww3rlflpg025xd26m9payz96u54x4fe3s2ktz39xqk67gzx";
pub const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n19604yflqggs9mk2z26mqygq43q2kr3n932egxx630svywd5mpxjsztfpvx";

pub const REWARDING_VALIDATOR_ADDRESS: &str = "n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy";

pub const NYXD_URL: &str = "https://rpc.nymtech.net";
pub const NYM_API: &str = "https://validator.nymtech.net/api/";
pub const NYXD_WS: &str = "wss://rpc.nymtech.net/websocket";
pub const EXPLORER_API: &str = "https://explorer.nymtech.net/api/";

// I'm making clippy mad on purpose, because that url HAS TO be updated and deployed before merging
pub const EXIT_POLICY_URL: &str =
    "https://nymtech.net/.wellknown/network-requester/exit-policy.txt";

#[cfg(feature = "network")]
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        NYXD_URL,
        Some(NYM_API),
        Some(NYXD_WS),
    )]
}

#[cfg(feature = "env")]
const DEFAULT_SUFFIX: &str = "_MAINNET_DEFAULT";

#[cfg(all(feature = "env", feature = "network"))]
fn set_var_to_default(var: &str, value: &str) {
    std::env::set_var(var, value);
    std::env::set_var(format!("{var}{DEFAULT_SUFFIX}"), "1")
}

#[cfg(all(feature = "env", feature = "network"))]
fn set_var_conditionally_to_default(var: &str, value: &str) {
    if std::env::var(var).is_err() {
        set_var_to_default(var, value)
    }
}

#[cfg(feature = "env")]
pub fn uses_default(var: &str) -> bool {
    std::env::var(format!("{var}{DEFAULT_SUFFIX}")).is_ok()
}

#[cfg(feature = "env")]
pub fn read_var_if_not_default(var: &str) -> Option<String> {
    if uses_default(var) {
        None
    } else {
        std::env::var(var).ok()
    }
}

#[cfg(feature = "env")]
pub fn read_parsed_var_if_not_default<T: std::str::FromStr>(
    var: &str,
) -> Option<Result<T, T::Err>> {
    read_var_if_not_default(var)
        .as_deref()
        .map(std::str::FromStr::from_str)
}

#[cfg(all(feature = "env", feature = "network"))]
pub fn export_to_env() {
    use crate::var_names;

    set_var_to_default(var_names::CONFIGURED, "true");
    set_var_to_default(var_names::NETWORK_NAME, NETWORK_NAME);
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
    set_var_to_default(var_names::ECASH_CONTRACT_ADDRESS, ECASH_CONTRACT_ADDRESS);
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
    set_var_to_default(var_names::NYXD, NYXD_URL);
    set_var_to_default(var_names::NYM_API, NYM_API);
    set_var_to_default(var_names::NYXD_WEBSOCKET, NYXD_WS);
    set_var_to_default(var_names::EXPLORER_API, EXPLORER_API);
    set_var_to_default(var_names::EXIT_POLICY_URL, EXIT_POLICY_URL);
}

#[cfg(all(feature = "env", feature = "network"))]
pub fn export_to_env_if_not_set() {
    use crate::var_names;

    set_var_conditionally_to_default(var_names::CONFIGURED, "true");
    set_var_conditionally_to_default(var_names::NETWORK_NAME, NETWORK_NAME);
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
    set_var_conditionally_to_default(var_names::ECASH_CONTRACT_ADDRESS, ECASH_CONTRACT_ADDRESS);
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
    set_var_conditionally_to_default(var_names::NYXD, NYXD_URL);
    set_var_conditionally_to_default(var_names::NYM_API, NYM_API);
    set_var_conditionally_to_default(var_names::NYXD_WEBSOCKET, NYXD_WS);
    set_var_conditionally_to_default(var_names::EXPLORER_API, EXPLORER_API);
    set_var_conditionally_to_default(var_names::EXIT_POLICY_URL, EXIT_POLICY_URL);
}
