// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::block_signing::types::EpochSigningResults;
use crate::rewarder::epoch::Epoch;
use nym_validator_client::nyxd::module_traits::staking;
use nym_validator_client::nyxd::{PageRequest, StakingQueryClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use nyxd_scraper::NyxdScraper;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Range;
use tracing::info;

pub(crate) mod types;

pub struct EpochSigning {
    pub(crate) rpc_client: QueryHttpRpcNyxdClient,
    pub(crate) nyxd_scraper: NyxdScraper,
}

impl EpochSigning {
    async fn get_voting_power(
        &self,
        address: &str,
        height_range: Range<i64>,
    ) -> Result<Option<i64>, NymRewarderError> {
        for height in height_range {
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
        if let Some(validators) = self.rpc_client.historical_info(height).await?.hist {
            Ok(validators.valset)
        } else {
            let mut page_request = None;
            let mut response = Vec::new();

            loop {
                let mut res = self
                    .rpc_client
                    .validators("".to_string(), page_request)
                    .await?;
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
        let epoch_start = current_epoch.start;
        let epoch_end = current_epoch.end;
        let first_block = self
            .nyxd_scraper
            .storage
            .get_first_block_height_after(epoch_start)
            .await?
            .unwrap_or_default();
        let last_block = self
            .nyxd_scraper
            .storage
            .get_last_block_height_before(epoch_end)
            .await?
            .unwrap_or_default();

        // each validator MUST be online at some point during the first 20 blocks, otherwise they're not getting anything.
        let vp_range_end = min(first_block + 20, last_block);
        let vp_range = first_block..vp_range_end;

        let mut total_vp = 0;
        let mut signed_in_epoch = HashMap::new();

        // for each validator, with a valid voting power, get number of signed blocks in the rewarding epoch
        for validator in validators {
            let Some(vp) = self
                .get_voting_power(&validator.consensus_address, vp_range.clone())
                .await?
            else {
                continue;
            };
            total_vp += vp;

            let signed = self
                .nyxd_scraper
                .storage
                .get_signed_between_times(&validator.consensus_address, epoch_start, epoch_end)
                .await?;
            signed_in_epoch.insert(validator, (signed, vp));
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
