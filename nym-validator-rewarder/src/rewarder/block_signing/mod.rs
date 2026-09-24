// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::block_signing::types::{EpochSigningResults, RawValidatorResult};
use crate::rewarder::epoch::Epoch;
use crate::rewarder::nyxd_client::NyxdClient;
use nym_validator_client::nyxd::module_traits::staking;
use nym_validator_client::nyxd::{AccountId, PageRequest};
use nyxd_scraper_sqlite::SqliteNyxdScraper;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Range;
use tracing::{debug, error, info, trace, warn};

pub(crate) mod types;

/// The block window over which a validator must show voting power to stay eligible: the first 20
/// blocks of the epoch, or the whole epoch when it is shorter. The end is inclusive of
/// `last_block`, so even a single-block epoch samples its one block.
fn voting_power_window(first_block: i64, last_block: i64) -> Range<i64> {
    first_block..min(first_block + 20, last_block + 1)
}

pub struct EpochSigning {
    pub(crate) nyxd_client: NyxdClient,
    pub(crate) nyxd_scraper: SqliteNyxdScraper,
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
                .storage()
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
        // the live set holds every validator still in the staking store, including those jailed
        // or unbonding since the epoch; it is fetched first so that the historical entries,
        // appended after it, win when both describe the same validator. a failure of either
        // source is tolerated as long as the other still yields validators
        let mut validators = Vec::new();
        let mut page_request = None;
        loop {
            let mut res = match self.nyxd_client.validators(page_request).await {
                Ok(res) => res,
                Err(err) => {
                    warn!("failed to obtain the live validator set: {err}");
                    break;
                }
            };

            let num_results = res.validators.len();
            validators.append(&mut res.validators);

            let Some(pagination) = res.pagination else {
                break;
            };
            if pagination.next_key.is_empty() || num_results == 0 {
                break;
            }

            page_request = Some(PageRequest {
                key: pagination.next_key,
                offset: 0,
                limit: 0,
                count_total: false,
                reverse: false,
            });
        }

        // the historical set is the bonded set at the epoch's last height
        match self.nyxd_client.historical_info(height).await {
            Ok(info) => {
                if let Some(hist) = info.hist {
                    validators.extend(hist.valset);
                }
            }
            Err(err) => {
                warn!("failed to obtain historical validator info for height {height}: {err}")
            }
        }

        // both sources failing (or genuinely empty) would otherwise settle a zero-reward epoch
        // silently; surface it so the epoch is flagged rather than paid out as nothing
        if validators.is_empty() {
            return Err(NymRewarderError::NoValidatorsToReward);
        }

        Ok(validators)
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

        let validators = self
            .nyxd_scraper
            .storage()
            .get_all_known_validators()
            .await?;
        debug!("retrieved {} known validators", validators.len());

        let epoch_start = current_epoch.start_time;
        let epoch_end = current_epoch.end_time;

        let Some(first_block) = self
            .nyxd_scraper
            .storage()
            .get_first_block_height_after(epoch_start)
            .await?
        else {
            return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                epoch: current_epoch,
            });
        };

        let Some(last_block) = self
            .nyxd_scraper
            .storage()
            .get_last_block_height_before(epoch_end)
            .await?
        else {
            return Err(NymRewarderError::NoBlocksProcessedInEpoch {
                epoch: current_epoch,
            });
        };

        // each validator MUST be online at some point during the first 20 blocks (or the whole
        // epoch if it is shorter), otherwise they're not getting anything.
        let vp_range = voting_power_window(first_block, last_block);

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
                error!(
                    "failed to obtain voting power for validator {addr} for any block between heights {vp_range:?} - there were no stored pre-commits for that validator."
                );
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
                .storage()
                .get_signed_between_times(&validator.consensus_address, epoch_start, epoch_end)
                .await?;
            signed_in_epoch.insert(validator, RawValidatorResult::new(signed, vp, whitelisted));
        }

        let total = self
            .nyxd_scraper
            .storage()
            .get_blocks_between(epoch_start, epoch_end)
            .await?;

        let details = self.get_validator_details(last_block).await?;

        EpochSigningResults::construct(total, total_vp, signed_in_epoch, details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voting_power_window_covers_the_first_20_blocks_of_a_long_epoch() {
        // a long epoch is capped at 20 blocks regardless of its length
        assert_eq!(voting_power_window(100, 1000), 100..120);
    }

    #[test]
    fn voting_power_window_includes_the_last_block_of_a_short_epoch() {
        // a short epoch samples all of its blocks, last one included
        assert_eq!(voting_power_window(100, 104), 100..105);
    }

    #[test]
    fn voting_power_window_of_a_single_block_epoch_samples_that_block() {
        // first_block == last_block must still yield a non-empty window
        let window = voting_power_window(100, 100);
        assert_eq!(window, 100..101);
        assert!(!window.is_empty());
    }
}
