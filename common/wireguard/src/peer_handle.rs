// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::peer_controller::PeerControlRequest;
use crate::peer_storage_manager::{CachedPeerManager, PeerInformation};
use defguard_wireguard_rs::{host::Host, key::Key};
use futures::channel::oneshot;
use nym_credential_verification::bandwidth_storage_manager::BandwidthStorageManager;
use nym_task::TaskClient;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

pub(crate) type SharedBandwidthStorageManager = Arc<RwLock<BandwidthStorageManager>>;

pub struct PeerHandle {
    public_key: Key,
    host_information: Arc<RwLock<Host>>,
    cached_peer: CachedPeerManager,
    bandwidth_storage_manager: SharedBandwidthStorageManager,
    request_tx: mpsc::Sender<PeerControlRequest>,
    timeout_check_interval: IntervalStream,
    task_client: TaskClient,
}

impl PeerHandle {
    pub fn new(
        public_key: Key,
        host_information: Arc<RwLock<Host>>,
        cached_peer: CachedPeerManager,
        bandwidth_storage_manager: SharedBandwidthStorageManager,
        request_tx: mpsc::Sender<PeerControlRequest>,
        task_client: &TaskClient,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let mut task_client = task_client.fork(format!("peer_{public_key}"));
        task_client.disarm();
        PeerHandle {
            public_key,
            host_information,
            cached_peer,
            bandwidth_storage_manager,
            request_tx,
            timeout_check_interval,
            task_client,
        }
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
            .success;
        Ok(success)
    }

    fn compute_spent_bandwidth(
        kernel_peer: PeerInformation,
        cached_peer: PeerInformation,
    ) -> Option<u64> {
        let kernel_total = kernel_peer
            .rx_bytes
            .checked_add(kernel_peer.tx_bytes)
            .or_else(|| {
                tracing::error!(
                    "Overflow on kernel adding bytes: {} + {}",
                    kernel_peer.rx_bytes,
                    kernel_peer.tx_bytes
                );
                None
            })?;
        let cached_total = cached_peer
            .rx_bytes
            .checked_add(cached_peer.tx_bytes)
            .or_else(|| {
                tracing::error!(
                    "Overflow on cached adding bytes: {} + {}",
                    cached_peer.rx_bytes,
                    cached_peer.tx_bytes
                );
                None
            })?;

        kernel_total.checked_sub(cached_total).or_else(|| {
            tracing::error!("Overflow on spent bandwidth subtraction: kernel - cached = {kernel_total} - {cached_total}");
            None
        })
    }

    async fn active_peer(&mut self, kernel_peer: PeerInformation) -> Result<bool, Error> {
        let Some(cached_peer) = self.cached_peer.get_peer() else {
            log::debug!(
                "Peer {:?} not in storage anymore, shutting down handle",
                self.public_key
            );
            return Ok(false);
        };

        let spent_bandwidth = Self::compute_spent_bandwidth(kernel_peer, cached_peer)
            .unwrap_or_default()
            .try_into()
            .inspect_err(|err| tracing::error!("Could not convert from u64 to i64: {err:?}"))
            .unwrap_or_default();

        self.cached_peer.update(kernel_peer);

        if spent_bandwidth > 0
            && self
                .bandwidth_storage_manager
                .write()
                .await
                .try_use_bandwidth(spent_bandwidth)
                .await
                .is_err()
        {
            tracing::debug!(
                "Peer {} is out of bandwidth, removing it",
                self.public_key.to_string()
            );
            let success = self.remove_peer().await?;
            self.cached_peer.remove_peer();
            return Ok(!success);
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
            log::debug!(
                "Peer {:?} is not active anymore, shutting down handle",
                self.public_key
            );
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub async fn run(&mut self) {
        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    match self.continue_checking().await {
                        Ok(true) => continue,
                        Ok(false) => return,
                        Err(err) => {
                            match self.remove_peer().await {
                                Ok(true) => {
                                    tracing::debug!("Removed peer due to error {err}");
                                    return;
                                }
                                _ => {
                                    tracing::warn!("Could not remove peer yet, we'll try again later. If this message persists, the gateway might need to be restarted");
                                    continue;
                                }
                            }
                        },
                    }
                }

                _ = self.task_client.recv() => {
                    log::trace!("PeerHandle: Received shutdown");
                    if let Err(e) = self.bandwidth_storage_manager.write().await.sync_storage_bandwidth().await {
                        log::error!("Storage sync failed - {e}, unaccounted bandwidth might have been consumed");
                    }

                    log::trace!("PeerHandle: Finished shutdown");
                }
            }
        }
    }
}
