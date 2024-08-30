// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::error::{EcashError, Result};
use crate::ecash::helpers::CachedImmutableEpochItem;
use crate::{ecash, nyxd};
use nym_coconut_dkg_common::types::{Epoch, EpochId};
use nym_dkg::Threshold;
use nym_validator_client::EcashApiClient;
use std::cmp::min;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockWriteGuard};

#[async_trait]
pub trait APICommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId>;

    async fn ecash_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>>;

    async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<Threshold>;

    async fn dkg_in_progress(&self) -> Result<bool>;
}

struct CachedEpoch {
    valid_until: OffsetDateTime,
    current_epoch: Epoch,
}

impl Default for CachedEpoch {
    fn default() -> Self {
        CachedEpoch {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            current_epoch: Epoch::default(),
        }
    }
}

impl CachedEpoch {
    fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    fn update(&mut self, epoch: Epoch) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        let validity_duration = if let Some(epoch_finish) = epoch.deadline {
            let state_end =
                OffsetDateTime::from_unix_timestamp(epoch_finish.seconds() as i64).unwrap();
            let until_epoch_state_end = state_end - now;
            // make it valid until the next epoch transition or next 5min, whichever is smaller
            min(until_epoch_state_end, 5 * time::Duration::MINUTE)
        } else {
            5 * time::Duration::MINUTE
        };

        self.valid_until = now + validity_duration;
        self.current_epoch = epoch;

        Ok(())
    }
}

pub(crate) struct QueryCommunicationChannel {
    nyxd_client: nyxd::Client,

    epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,
    cached_epoch: RwLock<CachedEpoch>,
    threshold_values: CachedImmutableEpochItem<Threshold>,
}

impl QueryCommunicationChannel {
    pub fn new(nyxd_client: nyxd::Client) -> Self {
        QueryCommunicationChannel {
            nyxd_client,
            epoch_clients: Default::default(),
            cached_epoch: Default::default(),
            threshold_values: Default::default(),
        }
    }

    async fn update_epoch_cache(&self) -> Result<RwLockWriteGuard<CachedEpoch>> {
        let mut guard = self.cached_epoch.write().await;

        let epoch = ecash::client::Client::get_current_epoch(&self.nyxd_client).await?;

        guard.update(epoch)?;
        Ok(guard)
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(guard.current_epoch.epoch_id);
        }

        // update cache
        drop(guard);
        let guard = self.update_epoch_cache().await?;

        return Ok(guard.current_epoch.epoch_id);
    }

    // TODO: perhaps this should be returning a ReadGuard instead?
    async fn ecash_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>> {
        self.epoch_clients
            .get_or_init(epoch_id, || async {
                self.nyxd_client
                    .get_registered_ecash_clients(epoch_id)
                    .await
            })
            .await
            .map(|guard| guard.clone())
    }

    async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<Threshold> {
        self.threshold_values
            .get_or_init(epoch_id, || async {
                if let Some(threshold) =
                    ecash::client::Client::get_epoch_threshold(&self.nyxd_client, epoch_id).await?
                {
                    Ok(threshold)
                } else {
                    Err(EcashError::UnavailableThreshold { epoch_id })
                }
            })
            .await
            .map(|t| *t)
    }

    async fn dkg_in_progress(&self) -> Result<bool> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(!guard.current_epoch.state.is_in_progress());
        }

        // update cache
        drop(guard);
        let guard = self.update_epoch_cache().await?;

        return Ok(!guard.current_epoch.state.is_in_progress());
    }
}
