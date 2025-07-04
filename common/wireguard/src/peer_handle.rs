// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::peer_controller::PeerControlRequest;
use crate::peer_storage_manager::PeerStorageManager;
use defguard_wireguard_rs::host::Peer;
use defguard_wireguard_rs::{host::Host, key::Key};
use futures::channel::oneshot;
use nym_authenticator_requests::latest::registration::BANDWIDTH_CAP_PER_DAY;
use nym_credential_verification::bandwidth_storage_manager::BandwidthStorageManager;
use nym_gateway_storage::models::WireguardPeer;
use nym_task::TaskClient;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

pub(crate) type SharedBandwidthStorageManager = Arc<RwLock<BandwidthStorageManager>>;

pub struct PeerHandle {
    public_key: Key,
    host_information: Arc<RwLock<Host>>,
    peer_storage_manager: PeerStorageManager,
    bandwidth_storage_manager: Option<SharedBandwidthStorageManager>,
    request_tx: mpsc::Sender<PeerControlRequest>,
    timeout_check_interval: IntervalStream,
    task_client: TaskClient,
}

impl PeerHandle {
    pub fn new(
        public_key: Key,
        host_information: Arc<RwLock<Host>>,
        peer_storage_manager: PeerStorageManager,
        bandwidth_storage_manager: Option<SharedBandwidthStorageManager>,
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
            peer_storage_manager,
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

    fn compute_spent_bandwidth(kernel_peer: &Peer, storage_peer: &WireguardPeer) -> Option<u64> {
        let storage_peer_rx_bytes = u64::try_from(storage_peer.rx_bytes)
            .inspect_err(|e| tracing::error!("Storage rx bytes could not be converted: {e}"))
            .ok()?;
        let storage_peer_tx_bytes = u64::try_from(storage_peer.tx_bytes)
            .inspect_err(|e| tracing::error!("Storage tx bytes could not be converted: {e}"))
            .ok()?;

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
        let storage_total = storage_peer_rx_bytes
            .checked_add(storage_peer_tx_bytes)
            .or_else(|| {
                tracing::error!("Overflow on storage adding bytes: {storage_peer_rx_bytes} + {storage_peer_tx_bytes}");
                None
            })?;

        kernel_total.checked_sub(storage_total).or_else(|| {
            tracing::error!("Overflow on spent bandwidth subtraction: kernel - storage = {kernel_total} - {storage_total}");
            None
        })
    }

    async fn active_peer(&mut self, kernel_peer: &Peer) -> Result<bool, Error> {
        let Some(storage_peer) = self.peer_storage_manager.get_peer() else {
            log::debug!(
                "Peer {:?} not in storage anymore, shutting down handle",
                self.public_key
            );
            return Ok(false);
        };

        if let Some(bandwidth_manager) = &self.bandwidth_storage_manager {
            let spent_bandwidth = Self::compute_spent_bandwidth(kernel_peer, &storage_peer)
                .unwrap_or_else(|| {
                    // if gateway restarted, the kernel values restart from 0
                    // and we should restart from 0 in storage as well
                    if let Some(peer_information) =
                        self.peer_storage_manager.peer_information.as_mut()
                    {
                        peer_information.force_sync = true;
                        peer_information.peer.rx_bytes = kernel_peer.rx_bytes;
                        peer_information.peer.tx_bytes = kernel_peer.tx_bytes;
                    }
                    0
                })
                .try_into()
                .map_err(|_| Error::InconsistentConsumedBytes)?;
            if spent_bandwidth > 0 {
                self.peer_storage_manager.update_trx(kernel_peer);
                if bandwidth_manager
                    .write()
                    .await
                    .try_use_bandwidth(spent_bandwidth)
                    .await
                    .is_err()
                {
                    tracing::debug!(
                        "Peer {} is out of bandwidth, removing it",
                        kernel_peer.public_key.to_string()
                    );
                    let success = self.remove_peer().await?;
                    self.peer_storage_manager.remove_peer();
                    return Ok(!success);
                }
            }
        } else {
            let spent_bandwidth = kernel_peer.rx_bytes + kernel_peer.tx_bytes;
            if spent_bandwidth >= BANDWIDTH_CAP_PER_DAY {
                log::debug!(
                    "Peer {} doesn't have bandwidth anymore, removing it",
                    self.public_key
                );
                let success = self.remove_peer().await?;
                return Ok(!success);
            }
        }

        Ok(true)
    }

    async fn continue_checking(&mut self) -> Result<bool, Error> {
        let Some(kernel_peer) = self
            .host_information
            .read()
            .await
            .peers
            .get(&self.public_key)
            .cloned()
        else {
            // the host information hasn't beed updated yet
            return Ok(true);
        };
        if !self.active_peer(&kernel_peer).await? {
            log::debug!(
                "Peer {:?} is not active anymore, shutting down handle",
                self.public_key
            );
            Ok(false)
        } else {
            // Update storage values
            self.peer_storage_manager.sync_storage_peer().await?;
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
                    if let Some(bandwidth_manager) = &self.bandwidth_storage_manager {
                        if let Err(e) = bandwidth_manager.write().await.sync_storage_bandwidth().await {
                            log::error!("Storage sync failed - {e}, unaccounted bandwidth might have been consumed");
                        }
                    }
                    log::trace!("PeerHandle: Finished shutdown");
                }
            }
        }
    }
}
