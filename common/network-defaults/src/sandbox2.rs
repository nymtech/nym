// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{DenomDetails, ValidatorDetails};

pub(crate) const BECH32_PREFIX: &str = "nymt";

pub const MIX_DENOM: DenomDetails = DenomDetails::new("unymt", "nymt", 6);
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyxt", "nyxt", 6);

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "";
pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("8e0DcFF7F3085235C32E845f3667aEB3f1e83133");
pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("E8883BAeF3869e14E4823F46662e81D4F7d2A81F");
pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "";

pub(crate) const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "";
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://sandbox2-validator.nymtech.net",
        Some("https://sandbox2-validator.nymtech.net/api"),
    )]
}
