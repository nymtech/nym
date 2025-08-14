// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::peer_controller::PeerControlRequest;
use crate::peer_storage_manager::{CachedPeerManager, PeerInformation};
use defguard_wireguard_rs::{host::Host, key::Key, net::IpAddrMask};
use futures::channel::oneshot;
use nym_credential_verification::OutOfBandwidthResultExt;
use nym_credential_verification::bandwidth_storage_manager::BandwidthStorageManager;
use nym_credential_verification::upgrade_mode::UpgradeModeStatus;
use nym_task::ShutdownToken;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::fmt::Display;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tracing::{debug, error, trace, warn};

#[derive(Clone)]
pub(crate) struct SharedBandwidthStorageManager {
    inner: Arc<RwLock<BandwidthStorageManager>>,
    allowed_ips: Vec<IpAddrMask>,
}

impl SharedBandwidthStorageManager {
    pub(crate) fn new(
        inner: Arc<RwLock<BandwidthStorageManager>>,
        allowed_ips: Vec<IpAddrMask>,
    ) -> Self {
        Self { inner, allowed_ips }
    }

    pub(crate) fn inner(&self) -> &RwLock<BandwidthStorageManager> {
        &self.inner
    }

    pub(crate) fn allowed_ips(&self) -> &[IpAddrMask] {
        &self.allowed_ips
    }
}

pub struct PeerHandle {
    public_key: Key,
    host_information: Arc<RwLock<Host>>,
    cached_peer: CachedPeerManager,
    bandwidth_storage_manager: SharedBandwidthStorageManager,
    request_tx: mpsc::Sender<PeerControlRequest>,
    timeout_check_interval: IntervalStream,

    /// Flag indicating whether the system is undergoing an upgrade and thus peers shouldn't be getting
    /// their bandwidth metered.
    upgrade_mode: UpgradeModeStatus,
    shutdown_token: ShutdownToken,
}

impl Display for PeerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "peer {}", self.public_key)
    }
}

impl PeerHandle {
    pub(crate) fn new(
        public_key: Key,
        host_information: Arc<RwLock<Host>>,
        cached_peer: CachedPeerManager,
        bandwidth_storage_manager: SharedBandwidthStorageManager,
        request_tx: mpsc::Sender<PeerControlRequest>,
        upgrade_mode: UpgradeModeStatus,
        shutdown_token: &ShutdownToken,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK));
        let shutdown_token = shutdown_token.clone();
        PeerHandle {
            public_key,
            host_information,
            cached_peer,
            bandwidth_storage_manager,
            request_tx,
            timeout_check_interval,
            upgrade_mode,
            shutdown_token,
        }
    }

    /// Attempt to use the specified amount of bandwidth and update internal cache.
    /// Returns the amount of bandwidth remaining
    async fn try_use_bandwidth(&self, spent: i64) -> nym_credential_verification::Result<i64> {
        self.bandwidth_storage_manager
            .inner
            .write()
            .await
            .try_use_bandwidth(spent)
            .await
    }

    async fn remove_peer(&self) -> Result<bool, Error> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(PeerControlRequest::RemovePeer {
                key: self.public_key.clone(),
                response_tx,
            })
            .await
            .map_err(|_| Error::Internal("peer controller shut down".to_string()))?;
        let success = response_rx
            .await
            .map_err(|_| Error::Internal("peer controller didn't respond".to_string()))?
            .inspect_err(|err| tracing::error!("Could not remove peer: {err:?}"))
            .is_ok();
        Ok(success)
    }

    async fn active_peer(&mut self, kernel_peer: PeerInformation) -> Result<bool, Error> {
        let Some(cached_peer) = self.cached_peer.get_peer() else {
            debug!("{self} not in storage anymore, shutting down handle");
            return Ok(false);
        };

        let spent_bandwidth = kernel_peer.consumed_kernel_bandwidth(&cached_peer);
        self.cached_peer.update(kernel_peer);

        if spent_bandwidth > 0 {
            trace!("{self} has used {spent_bandwidth} of bandwidth");
            if self.upgrade_mode.enabled() {
                debug!("we're in upgrade mode - {self} is not going to get its bandwidth deducted");
                return Ok(true);
            }

            // 'regular' flow
            if self
                .try_use_bandwidth(spent_bandwidth)
                .await
                .is_out_of_bandwidth()
            {
                debug!("{self} is out of bandwidth, removing it");
                let success = self.remove_peer().await?;
                self.cached_peer.remove_peer();
                return Ok(!success);
            }
        }

        Ok(true)
    }

    async fn continue_checking(&mut self) -> Result<bool, Error> {
        let kernel_peer = self
            .host_information
            .read()
            .await
            .peers
            .get(&self.public_key)
            .ok_or(Error::MissingClientKernelEntry(self.public_key.to_string()))?
            .into();
        if !self.active_peer(kernel_peer).await? {
            debug!("{self} is not active anymore, shutting down handle",);
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("PeerHandle: Received shutdown");
                    if let Err(e) = self.bandwidth_storage_manager.inner().write().await.sync_storage_bandwidth().await {
                        error!("Storage sync failed - {e}, unaccounted bandwidth might have been consumed");
                    }

                    trace!("PeerHandle: Finished shutdown");
                    break;
                }
                _ = self.timeout_check_interval.next() => {
                    match self.continue_checking().await {
                        Ok(true) => continue,
                        Ok(false) => return,
                        Err(err) => {
                            match self.remove_peer().await {
                                Ok(true) => {
                                    debug!("Removed peer due to error {err}");
                                    return;
                                }
                                _ => {
                                    warn!("Could not remove peer yet, we'll try again later. If this message persists, the gateway might need to be restarted");
                                    continue;
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
