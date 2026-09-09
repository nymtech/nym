// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::BandwidthFlushingBehaviourConfig;
use crate::ClientBandwidth;
use crate::error::*;
use nym_credentials::ecash::utils::ecash_today;
use nym_credentials_interface::{AvailableBandwidth, Bandwidth};
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use si_scale::helpers::bibytes2;
use time::{Date, OffsetDateTime, Time};
use tracing::*;

const FREE_TESTNET_BANDWIDTH_VALUE: Bandwidth = Bandwidth::new_unchecked(64 * 1024 * 1024 * 1024); // 64GB

/// Synthetic allowance seeded for an ephemeral session: half of `i64::MAX`, roughly four exabytes.
/// Deliberately absurd rather than merely generous, so that it stays unbounded in practice for
/// whatever an unmetered session is used for beyond the liveness probe that motivated it, while
/// leaving as much headroom again for any credit (the testnet grant, say) to land without
/// overflowing the counter.
const EPHEMERAL_BANDWIDTH_VALUE: i64 = i64::MAX / 2;

#[derive(Clone)]
pub struct BandwidthStorageManager {
    persistence: BandwidthPersistence,
    pub(crate) client_bandwidth: ClientBandwidth,
    pub(crate) bandwidth_cfg: BandwidthFlushingBehaviourConfig,
    pub(crate) only_coconut_credentials: bool,
}

impl BandwidthStorageManager {
    pub fn new(
        storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,
        client_bandwidth: ClientBandwidth,
        client_id: i64,
        bandwidth_cfg: BandwidthFlushingBehaviourConfig,
        only_coconut_credentials: bool,
    ) -> Self {
        BandwidthStorageManager {
            persistence: BandwidthPersistence::Persisted { storage, client_id },
            client_bandwidth,
            bandwidth_cfg,
            only_coconut_credentials,
        }
    }

    /// Create a manager for an unmetered session: it seeds a synthetic allowance ([`EPHEMERAL_BANDWIDTH_VALUE`])
    /// and never reads or writes storage, because it holds no handle to any and no client id to key
    /// rows by.
    ///
    /// Takes no configuration because none of it applies: the flushing thresholds only decide when
    /// to sync with a storage that isn't there, and there are no credentials to restrict.
    pub fn new_ephemeral() -> Self {
        BandwidthStorageManager {
            persistence: BandwidthPersistence::Ephemeral,
            client_bandwidth: ClientBandwidth::new(AvailableBandwidth {
                bytes: EPHEMERAL_BANDWIDTH_VALUE,
                // an ephemeral allowance is not purchased and cannot be topped up, so it must never
                // expire: an expiry would end the session rather than prompt a renewal
                expiration: OffsetDateTime::new_utc(Date::MAX, Time::MIDNIGHT),
            }),
            bandwidth_cfg: Default::default(),
            only_coconut_credentials: false,
        }
    }

    /// The storage this session's bandwidth is backed by.
    ///
    /// An ephemeral session has none, and spending a credential against it is a client error rather
    /// than an internal one: it has no ticket store to check for double spending and no rows to
    /// credit.
    pub(crate) fn storage(&self) -> Result<&(dyn BandwidthGatewayStorage + Send + Sync)> {
        match &self.persistence {
            BandwidthPersistence::Persisted { storage, .. } => Ok(&**storage),
            BandwidthPersistence::Ephemeral => Err(Error::UnmeteredSession),
        }
    }

    /// The storage-assigned id of the client owning this session's rows. See [`Self::storage`].
    pub(crate) fn client_id(&self) -> Result<i64> {
        match &self.persistence {
            BandwidthPersistence::Persisted { client_id, .. } => Ok(*client_id),
            BandwidthPersistence::Ephemeral => Err(Error::UnmeteredSession),
        }
    }

    pub fn client_bandwidth(&self) -> ClientBandwidth {
        self.client_bandwidth.clone()
    }

    pub async fn available_bandwidth(&self) -> i64 {
        self.client_bandwidth.available().await
    }

    async fn sync_expiration(&mut self) -> Result<()> {
        self.persistence
            .set_expiration(self.client_bandwidth.expiration().await)
            .await
    }

    pub async fn handle_claim_testnet_bandwidth(&mut self) -> Result<i64> {
        debug!("handling testnet bandwidth request");

        if self.only_coconut_credentials {
            return Err(Error::OnlyCoconutCredentials);
        }

        self.increase_bandwidth(FREE_TESTNET_BANDWIDTH_VALUE, ecash_today())
            .await?;
        let available_total = self.client_bandwidth.available().await;
        Ok(available_total)
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
        trace!(available = available_bi2, required = required_bi2);

        self.consume_bandwidth(required_bandwidth).await?;
        let remaining_bandwidth = self.client_bandwidth.available().await;
        Ok(remaining_bandwidth)
    }

    async fn expire_bandwidth(&mut self) -> Result<()> {
        // an ephemeral allowance cannot expire (see `new_ephemeral`), so this is unreachable for one;
        // zeroing it would kill the session with no way to replenish it
        if self.persistence.is_ephemeral() {
            return Ok(());
        }

        self.persistence.reset_bandwidth().await?;
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
    pub async fn sync_storage_bandwidth(&mut self) -> Result<()> {
        trace!("syncing client bandwidth with the underlying storage");

        let delta = self.client_bandwidth.delta_since_sync().await;

        // for an ephemeral session there is nothing to sync against, and resyncing would replace the
        // synthetic allowance with a stored value that does not exist
        if let Some(updated) = self.persistence.increase_bandwidth(delta).await? {
            self.client_bandwidth
                .resync_bandwidth_with_storage(updated)
                .await;
        }

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

/// Where a session's bandwidth lives.
enum BandwidthPersistence {
    /// A metered session, whose allowance is backed by its storage rows.
    Persisted {
        storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,

        /// storage-assigned id of the client those rows belong to
        client_id: i64,
    },

    /// An unmetered session carrying a synthetic allowance and persisting nothing. It holds neither
    /// a storage handle nor a client id, which is what makes "no read or write" a property of the
    /// type rather than a discipline every method has to keep.
    Ephemeral,
}

impl Clone for BandwidthPersistence {
    fn clone(&self) -> Self {
        match self {
            BandwidthPersistence::Persisted { storage, client_id } => {
                BandwidthPersistence::Persisted {
                    storage: dyn_clone::clone_box(&**storage),
                    client_id: *client_id,
                }
            }
            BandwidthPersistence::Ephemeral => BandwidthPersistence::Ephemeral,
        }
    }
}

impl BandwidthPersistence {
    fn is_ephemeral(&self) -> bool {
        matches!(self, BandwidthPersistence::Ephemeral)
    }

    async fn set_expiration(&self, expiration: OffsetDateTime) -> Result<()> {
        if let BandwidthPersistence::Persisted { storage, client_id } = self {
            storage.set_expiration(*client_id, expiration).await?;
        }
        Ok(())
    }

    async fn reset_bandwidth(&self) -> Result<()> {
        if let BandwidthPersistence::Persisted { storage, client_id } = self {
            storage.reset_bandwidth(*client_id).await?;
        }
        Ok(())
    }

    /// Credit the stored allowance, returning the new stored total, or `None` for an ephemeral
    /// session whose allowance is not backed by storage.
    async fn increase_bandwidth(&self, amount: i64) -> Result<Option<i64>> {
        match self {
            BandwidthPersistence::Persisted { storage, client_id } => {
                Ok(Some(storage.increase_bandwidth(*client_id, amount).await?))
            }
            BandwidthPersistence::Ephemeral => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_ephemeral_session_starts_with_an_allowance() {
        let manager = BandwidthStorageManager::new_ephemeral();

        assert_eq!(
            EPHEMERAL_BANDWIDTH_VALUE,
            manager.available_bandwidth().await
        );
    }

    // The allowance must not expire: nothing can replenish it, so an expiry would end the session.
    #[tokio::test]
    async fn an_ephemeral_allowance_never_expires() {
        let manager = BandwidthStorageManager::new_ephemeral();

        assert!(!manager.client_bandwidth.expired().await);
    }

    #[tokio::test]
    async fn an_ephemeral_session_consumes_its_allowance_without_storage() {
        let mut manager = BandwidthStorageManager::new_ephemeral();
        let allowance = manager.available_bandwidth().await;

        let remaining = manager
            .try_use_bandwidth(1024)
            .await
            .expect("an ephemeral session must not need storage to consume bandwidth");

        assert_eq!(allowance - 1024, remaining);
    }

    // Regression guard: syncing a session whose allowance is synthetic must leave it alone. Were it
    // to resync against storage it would adopt a stored total that does not exist, zeroing the
    // allowance and failing every subsequent packet with `OutOfBandwidth`.
    #[tokio::test]
    async fn syncing_an_ephemeral_session_leaves_its_allowance_intact() {
        let mut manager = BandwidthStorageManager::new_ephemeral();

        manager.try_use_bandwidth(1024).await.unwrap();
        let before_sync = manager.available_bandwidth().await;

        manager
            .sync_storage_bandwidth()
            .await
            .expect("an ephemeral session must sync to nothing rather than fail");

        assert_eq!(before_sync, manager.available_bandwidth().await);
    }

    // Consumption crosses the flushing threshold, which drives a sync on the ordinary path. Even
    // then the allowance must only ever go down by what was used.
    #[tokio::test]
    async fn an_ephemeral_session_survives_crossing_the_flush_threshold() {
        let mut manager = BandwidthStorageManager::new_ephemeral();
        let allowance = manager.available_bandwidth().await;

        let over_threshold = manager
            .bandwidth_cfg
            .client_bandwidth_max_delta_flushing_amount
            + 1;
        let remaining = manager.try_use_bandwidth(over_threshold).await.unwrap();

        assert_eq!(allowance - over_threshold, remaining);
    }

    #[tokio::test]
    async fn an_ephemeral_session_can_be_credited_without_storage() {
        let mut manager = BandwidthStorageManager::new_ephemeral();
        let allowance = manager.available_bandwidth().await;

        manager
            .increase_bandwidth(Bandwidth::new_unchecked(1024), ecash_today())
            .await
            .expect("crediting must not require storage");

        assert_eq!(allowance + 1024, manager.available_bandwidth().await);
    }

    // A credential presented on an unmetered session is a client error: there is no ticket store to
    // check against and no rows to credit.
    #[tokio::test]
    async fn an_ephemeral_session_exposes_no_storage_to_spend_against() {
        let manager = BandwidthStorageManager::new_ephemeral();

        assert!(matches!(manager.storage(), Err(Error::UnmeteredSession)));
        assert!(matches!(manager.client_id(), Err(Error::UnmeteredSession)));
    }
}
