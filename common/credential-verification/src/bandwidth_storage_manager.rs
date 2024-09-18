// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::*;
use crate::BandwidthFlushingBehaviourConfig;
use crate::ClientBandwidth;
use nym_credentials::ecash::utils::ecash_today;
use nym_credentials_interface::Bandwidth;
use nym_gateway_requests::ServerResponse;
use nym_gateway_storage::Storage;
use si_scale::helpers::bibytes2;
use time::OffsetDateTime;
use tracing::*;

const FREE_TESTNET_BANDWIDTH_VALUE: Bandwidth = Bandwidth::new_unchecked(64 * 1024 * 1024 * 1024); // 64GB

#[derive(Clone)]
pub struct BandwidthStorageManager<S> {
    pub(crate) storage: S,
    pub(crate) client_bandwidth: ClientBandwidth,
    pub(crate) client_id: i64,
    pub(crate) bandwidth_cfg: BandwidthFlushingBehaviourConfig,
    pub(crate) only_coconut_credentials: bool,
}

impl<S: Storage + Clone + 'static> BandwidthStorageManager<S> {
    pub fn new(
        storage: S,
        client_bandwidth: ClientBandwidth,
        client_id: i64,
        bandwidth_cfg: BandwidthFlushingBehaviourConfig,
        only_coconut_credentials: bool,
    ) -> Self {
        BandwidthStorageManager {
            storage,
            client_bandwidth,
            client_id,
            bandwidth_cfg,
            only_coconut_credentials,
        }
    }

    async fn sync_expiration(&mut self) -> Result<()> {
        self.storage
            .set_expiration(self.client_id, self.client_bandwidth.expiration().await)
            .await?;
        Ok(())
    }

    pub async fn handle_claim_testnet_bandwidth(&mut self) -> Result<ServerResponse> {
        debug!("handling testnet bandwidth request");

        if self.only_coconut_credentials {
            return Err(Error::OnlyCoconutCredentials);
        }

        self.increase_bandwidth(FREE_TESTNET_BANDWIDTH_VALUE, ecash_today())
            .await?;
        let available_total = self.client_bandwidth.available().await;

        Ok(ServerResponse::Bandwidth { available_total })
    }

    #[instrument(skip_all)]
    pub async fn try_use_bandwidth(&mut self, required_bandwidth: i64) -> Result<i64> {
        if self.client_bandwidth.expired().await {
            self.expire_bandwidth().await?;
        }
        let available_bandwidth = self.client_bandwidth.available().await;

        if available_bandwidth < required_bandwidth {
            return Err(Error::OutOfBandwidth {
                required: required_bandwidth,
                available: available_bandwidth,
            });
        }

        let available_bi2 = bibytes2(available_bandwidth as f64);
        let required_bi2 = bibytes2(required_bandwidth as f64);
        debug!(available = available_bi2, required = required_bi2);

        self.consume_bandwidth(required_bandwidth).await?;
        Ok(available_bandwidth)
    }

    async fn expire_bandwidth(&mut self) -> Result<()> {
        self.storage.reset_bandwidth(self.client_id).await?;
        self.client_bandwidth.expire_bandwidth().await;
        Ok(())
    }

    /// Decreases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to decrease the available bandwidth by.
    async fn consume_bandwidth(&mut self, amount: i64) -> Result<()> {
        self.client_bandwidth.decrease_bandwidth(amount).await;

        // since we're going to be operating on a fair use policy anyway, even if we crash and let extra few packets
        // through, that's completely fine
        if self.client_bandwidth.should_sync(self.bandwidth_cfg).await {
            self.sync_storage_bandwidth().await?;
        }

        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    async fn sync_storage_bandwidth(&mut self) -> Result<()> {
        trace!("syncing client bandwidth with the underlying storage");
        let updated = self
            .storage
            .increase_bandwidth(
                self.client_id,
                self.client_bandwidth.delta_since_sync().await,
            )
            .await?;

        self.client_bandwidth
            .resync_bandwidth_with_storage(updated)
            .await;
        Ok(())
    }

    /// Increases the amount of available bandwidth of the connected client by the specified value.
    ///
    /// # Arguments
    ///
    /// * `amount`: amount to increase the available bandwidth by.
    /// * `expiration` : the expiration date of that bandwidth
    pub async fn increase_bandwidth(
        &mut self,
        bandwidth: Bandwidth,
        expiration: OffsetDateTime,
    ) -> Result<()> {
        self.client_bandwidth
            .increase_bandwidth(bandwidth.value() as i64, expiration)
            .await;

        // any increases to bandwidth should get flushed immediately
        // (we don't want to accidentally miss somebody claiming a gigabyte voucher)
        self.sync_expiration().await?;
        self.sync_storage_bandwidth().await?;
        Ok(())
    }
}
