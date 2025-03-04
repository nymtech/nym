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
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

pub(crate) type SharedBandwidthStorageManager = Arc<RwLock<BandwidthStorageManager>>;
const AUTO_REMOVE_AFTER: Duration = Duration::from_secs(60 * 60 * 24 * 30); // 30 days

pub struct PeerHandle {
    public_key: Key,
    host_information: Arc<RwLock<Host>>,
    peer_storage_manager: PeerStorageManager,
    bandwidth_storage_manager: Option<SharedBandwidthStorageManager>,
    request_tx: mpsc::Sender<PeerControlRequest>,
    timeout_check_interval: IntervalStream,
    task_client: TaskClient,
    startup_timestamp: SystemTime,
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
            startup_timestamp: SystemTime::now(),
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

    async fn active_peer(
        &mut self,
        storage_peer: &WireguardPeer,
        kernel_peer: &Peer,
    ) -> Result<bool, Error> {
        if let Some(bandwidth_manager) = &self.bandwidth_storage_manager {
            let spent_bandwidth = (kernel_peer.rx_bytes + kernel_peer.tx_bytes)
                .checked_sub(storage_peer.rx_bytes as u64 + storage_peer.tx_bytes as u64)
                .ok_or(Error::InconsistentConsumedBytes)?
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
                    let success = self.remove_peer().await?;
                    self.peer_storage_manager.remove_peer();
                    return Ok(!success);
                }
            }
        } else {
            if SystemTime::now().duration_since(self.startup_timestamp)? >= AUTO_REMOVE_AFTER {
                log::debug!(
                    "Peer {} has been present for 30 days, removing it",
                    self.public_key.to_string()
                );
                let success = self.remove_peer().await?;
                return Ok(!success);
            }
            let spent_bandwidth = kernel_peer.rx_bytes + kernel_peer.tx_bytes;
            if spent_bandwidth >= BANDWIDTH_CAP_PER_DAY {
                log::debug!(
                    "Peer {} doesn't have bandwidth anymore, removing it",
                    self.public_key.to_string()
                );
                let success = self.remove_peer().await?;
                return Ok(!success);
            }
        }

        Ok(true)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    let Some(kernel_peer) = self
                        .host_information
                        .read()
                        .await
                        .peers
                        .get(&self.public_key)
                        .cloned() else {
                            // the host information hasn't beed updated yet
                            continue;
                        };
                    let Some(storage_peer) = self.peer_storage_manager.get_peer() else {
                        log::debug!("Peer {:?} not in storage anymore, shutting down handle", self.public_key);
                        return Ok(());
                    };
                    if !self.active_peer(&storage_peer, &kernel_peer).await? {
                        log::debug!("Peer {:?} doesn't have bandwidth anymore, shutting down handle", self.public_key);
                        return Ok(());
                    } else {
                        // Update storage values
                        self.peer_storage_manager.sync_storage_peer().await?;
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
        Ok(())
    }
}
