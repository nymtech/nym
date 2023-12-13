// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use nym_validator_client::nyxd::AccountId;
use nyxd_scraper::constants::BECH32_PREFIX;

pub(crate) fn consensus_address_to_account(
    consensus_address: &str,
) -> Result<AccountId, NymRewarderError> {
    let consensus_addr: AccountId =
        consensus_address
            .parse()
            .map_err(|source| NymRewarderError::MalformedBech32Address {
                consensus_address: consensus_address.to_string(),
                source,
            })?;
    AccountId::new(BECH32_PREFIX, &consensus_addr.to_bytes()).map_err(|source| {
        NymRewarderError::MalformedBech32Address {
            consensus_address: consensus_address.to_string(),
            source,
        }
    })
}
