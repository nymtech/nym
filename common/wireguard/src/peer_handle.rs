// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::peer_controller::PeerControlRequest;
use defguard_wireguard_rs::{host::Host, key::Key};
use futures::channel::oneshot;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth,
};
use nym_gateway_storage::models::WireguardPeer;
use nym_gateway_storage::Storage;
use nym_task::TaskClient;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

pub struct PeerHandle<St> {
    storage: St,
    public_key: Key,
    host_information: Arc<RwLock<Host>>,
    bandwidth_manager: Option<BandwidthStorageManager<St>>,
    request_tx: mpsc::UnboundedSender<PeerControlRequest>,
    timeout_check_interval: IntervalStream,
    task_client: TaskClient,
}

impl<St: Storage + Clone + 'static> PeerHandle<St> {
    pub async fn new(
        storage: St,
        public_key: Key,
        host_information: Arc<RwLock<Host>>,
        request_tx: mpsc::UnboundedSender<PeerControlRequest>,
        task_client: TaskClient,
    ) -> Result<Self, Error> {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let bandwidth_manager = if let Some(client_id) = storage
            .get_wireguard_peer(&public_key.to_string())
            .await?
            .ok_or(Error::MissingClientBandwidthEntry)?
            .client_id
        {
            let bandwidth = storage
                .get_available_bandwidth(client_id)
                .await?
                .ok_or(Error::MissingClientBandwidthEntry)?;
            Some(BandwidthStorageManager::new(
                storage.clone(),
                ClientBandwidth::new(bandwidth.into()),
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            ))
        } else {
            None
        };
        Ok(PeerHandle {
            storage,
            public_key,
            host_information,
            bandwidth_manager,
            request_tx,
            timeout_check_interval,
            task_client,
        })
    }

    async fn active_peer(&mut self, storage_peer: WireguardPeer) -> Result<bool, Error> {
        let kernel_peer = self
            .host_information
            .read()
            .await
            .peers
            .get(&self.public_key)
            .ok_or(Error::PeerMismatch)?
            .clone();
        let spent_bandwidth = (kernel_peer.rx_bytes + kernel_peer.tx_bytes)
            .checked_sub(storage_peer.rx_bytes as u64 + storage_peer.tx_bytes as u64)
            .ok_or(Error::InconsistentConsumedBytes)?
            .try_into()
            .map_err(|_| Error::InconsistentConsumedBytes)?;
        if let Some(bandwidth_manager) = self.bandwidth_manager.as_mut() {
            if bandwidth_manager
                .try_use_bandwidth(spent_bandwidth)
                .await
                .is_err()
            {
                log::debug!(
                    "Peer {} doesn't have bandwidth anymore, removing it",
                    self.public_key.to_string()
                );
                let (response_tx, response_rx) = oneshot::channel();
                self.request_tx
                    .send(PeerControlRequest::RemovePeer {
                        key: self.public_key.clone(),
                        response_tx,
                    })
                    .map_err(|_| Error::InternalError("peer controller shut down".to_string()))?;
                let success = response_rx
                    .await
                    .map_err(|_| {
                        Error::InternalError("peer controller didn't respond".to_string())
                    })?
                    .success;
                return Ok(!success);
            }
        }

        Ok(true)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        while !self.task_client.is_shutdown() {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    let Some(peer) = self.storage.get_wireguard_peer(&self.public_key.to_string()).await? else {
                        log::debug!("Peer {:?} not in storage anymore, shutting down handle", self.public_key);
                        return Ok(());
                    };
                    if !self.active_peer(peer).await? {
                        log::debug!("Peer {:?} doesn't have bandwidth anymore, shutting down handle", self.public_key);
                        return Ok(());
                    }
                }

                _ = self.task_client.recv() => {
                    log::trace!("PeerHandle: Received shutdown");
                }
            }
        }
        Ok(())
    }
}
