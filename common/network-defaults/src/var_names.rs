// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Environment variable that, if set, shows the environment is currently configured
pub const CONFIGURED: &str = "CONFIGURED";

pub const BECH32_PREFIX: &str = "BECH32_PREFIX";
pub const MIX_DENOM: &str = "MIX_DENOM";
pub const MIX_DENOM_DISPLAY: &str = "MIX_DENOM_DISPLAY";
pub const STAKE_DENOM: &str = "STAKE_DENOM";
pub const STAKE_DENOM_DISPLAY: &str = "STAKE_DENOM_DISPLAY";
pub const DENOMS_EXPONENT: &str = "DENOMS_EXPONENT";
pub const MIXNET_CONTRACT_ADDRESS: &str = "MIXNET_CONTRACT_ADDRESS";
pub const VESTING_CONTRACT_ADDRESS: &str = "VESTING_CONTRACT_ADDRESS";
pub const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = "BANDWIDTH_CLAIM_CONTRACT_ADDRESS";
pub const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str = "COCONUT_BANDWIDTH_CONTRACT_ADDRESS";
pub const GROUP_CONTRACT_ADDRESS: &str = "GROUP_CONTRACT_ADDRESS";
pub const MULTISIG_CONTRACT_ADDRESS: &str = "MULTISIG_CONTRACT_ADDRESS";
pub const COCONUT_DKG_CONTRACT_ADDRESS: &str = "COCONUT_DKG_CONTRACT_ADDRESS";
pub const REWARDING_VALIDATOR_ADDRESS: &str = "REWARDING_VALIDATOR_ADDRESS";
pub const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "STATISTICS_SERVICE_DOMAIN_ADDRESS";
pub const NYXD: &str = "NYXD";
pub const NYM_API: &str = "NYM_API";

pub const DKG_TIME_CONFIGURATION: &str = "DKG_TIME_CONFIGURATION";

// we don't want to explicitly tag those with `#[deprecated]` because then our CI would be red and sad : (
pub const DEPRECATED_NYMD_VALIDATOR: &str = "NYMD_VALIDATOR";
pub const DEPRECATED_API_VALIDATOR: &str = "API_VALIDATOR";
