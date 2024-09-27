// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::block_signing::types::{EpochSigningResults, RawValidatorResult};
use crate::rewarder::epoch::Epoch;
use crate::rewarder::nyxd_client::NyxdClient;
use nym_validator_client::nyxd::module_traits::staking;
use nym_validator_client::nyxd::{AccountId, PageRequest};
use nyxd_scraper::NyxdScraper;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Range;
use tracing::{debug, error, info, trace, warn};

pub(crate) mod types;

pub struct EpochSigning {
    pub(crate) nyxd_client: NyxdClient,
    pub(crate) nyxd_scraper: NyxdScraper,
    pub(crate) whitelist: Vec<AccountId>,
}

impl EpochSigning {
    async fn get_voting_power(
        &self,
        address: &str,
        height_range: Range<i64>,
    ) -> Result<Option<i64>, NymRewarderError> {
        for height in height_range {
            trace!("attempting to get pre-commit for {address} at height {height}");
            if let Some(precommit) = self
                .nyxd_scraper
                .storage
                .get_precommit(address, height)
                .await?
            {
                return Ok(Some(precommit.voting_power));
            }
        }

        Ok(None)
    }

    // TODO: eventually this will be replaced by scraping the data from the staking module in the scraper itself
    async fn get_validator_details(
        &self,
        height: i64,
    ) -> Result<Vec<staking::Validator>, NymRewarderError> {
        // first attempt to get it via the historical info.
        // if that fails, attempt to use current block information to at least get **something**
        if let Some(validators) = self.nyxd_client.historical_info(height).await?.hist {
            Ok(validators.valset)
        } else {
            let mut page_request = None;
            let mut response = Vec::new();

            loop {
                let mut res = self.nyxd_client.validators(page_request).await?;
                response.append(&mut res.validators);

                let Some(pagination) = res.pagination else {
                    break;
                };

                page_request = Some(PageRequest {
                    key: pagination.next_key,
                    offset: 0,
                    limit: 0,
                    count_total: false,
                    reverse: false,
                });
            }

            Ok(response)
        }
    }

    pub(crate) async fn get_signed_blocks_results(
        &self,
        current_epoch: Epoch,
    ) -> Result<EpochSigningResults, NymRewarderError> {
        info!(
            "looking up block signers for epoch {} ({} - {})",
            current_epoch.id,
            current_epoch.start_rfc3339(),
            current_epoch.end_rfc3339()
        );

        let validators = self.nyxd_scraper.storage.get_all_known_validators().await?;
        debug!("retrieved {} known validators", validators.len());

        let epoch_start = current_epoch.start_time;
        let epoch_end = current_epoch.end_time;

        let Some(first_block) = self
            .nyxd_scraper
            .storage
            .get_first_block_height_after(epoch_start)
            .await?
        else {
            return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                epoch: current_epoch,
            });
        };

        let Some(last_block) = self
            .nyxd_scraper
            .storage
            .get_last_block_height_before(epoch_end)
            .await?
        else {
            return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                epoch: current_epoch,
            });
        };

        // each validator MUST be online at some point during the first 20 blocks, otherwise they're not getting anything.
        let vp_range_end = min(first_block + 20, last_block);
        let vp_range = first_block..vp_range_end;

        let mut total_vp = 0;
        let mut signed_in_epoch = HashMap::new();

        // for each validator, with a valid voting power, get number of signed blocks in the rewarding epoch
        for validator in validators {
            let addr = &validator.consensus_address;
            debug!("getting voting power and signed blocks of {addr}");

            let Some(vp) = self
                .get_voting_power(&validator.consensus_address, vp_range.clone())
                .await?
            else {
                error!("failed to obtain voting power for validator {addr} for any block between heights {vp_range:?} - there were no stored pre-commits for that validator.");
                continue;
            };

            let cons_address = &validator.consensus_address;
            // if this validator is NOT whitelisted, do not increase the total VP
            let whitelisted = if let Ok(parsed) = cons_address.parse() {
                if self.whitelist.contains(&parsed) {
                    debug!("{cons_address} is on the whitelist");
                    total_vp += vp;
                    true
                } else {
                    warn!("{cons_address} is not a valid consensus address");
                    false
                }
            } else {
                debug!("{cons_address} is not on the whitelist");
                false
            };

            let signed = self
                .nyxd_scraper
                .storage
                .get_signed_between_times(&validator.consensus_address, epoch_start, epoch_end)
                .await?;
            signed_in_epoch.insert(validator, RawValidatorResult::new(signed, vp, whitelisted));
        }

        let total = self
            .nyxd_scraper
            .storage
            .get_blocks_between(epoch_start, epoch_end)
            .await?;

        let details = self.get_validator_details(last_block).await?;

        EpochSigningResults::construct(total, total_vp, signed_in_epoch, details)
    }
}
