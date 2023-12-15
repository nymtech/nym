// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use cosmwasm_std::Decimal;
use nym_validator_client::nyxd::{AccountId, PublicKey};
use nyxd_scraper::constants::{BECH32_CONSENSUS_ADDRESS_PREFIX, BECH32_PREFIX};
use sha2::{Digest, Sha256};

pub(crate) fn consensus_pubkey_to_address(
    pubkey: PublicKey,
) -> Result<AccountId, NymRewarderError> {
    let digest = Sha256::digest(pubkey.to_bytes()).to_vec();

    // TODO: make those configurable, etc
    AccountId::new(BECH32_CONSENSUS_ADDRESS_PREFIX, &digest[..20]).map_err(|source| {
        NymRewarderError::MalformedConsensusPublicKey {
            public_key: pubkey.to_string(),
            source,
        }
    })
}

// it's just a matter of swapping bech32 prefixes and recalculating the checksum
pub(crate) fn operator_account_to_owner_account(
    operator_address: &AccountId,
) -> Result<AccountId, NymRewarderError> {
    AccountId::new(BECH32_PREFIX, &operator_address.to_bytes()).map_err(|source| {
        NymRewarderError::MalformedBech32Address {
            operator_address: operator_address.to_string(),
            source,
        }
    })
}
