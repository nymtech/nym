// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorDetails;

pub(crate) const BECH32_PREFIX: &str = "punk";
pub(crate) const DENOM: &str = "upunk";

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "punk1jld76tqw4wnpfenmay2xkv86nr3j0w426eka82";
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "punk1v9qauwdq5terag6uvfsdytcs2d0sdmfdy7hgk3";

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![
        ValidatorDetails::new(
            "https://testnet-milhon-validator1.nymtech.net",
            Some("https://testnet-milhon-validator1.nymtech.net/api"),
        ),
        ValidatorDetails::new("https://testnet-milhon-validator2.nymtech.net", None),
    ]
}
