// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    WireguardInterfaceApi,
};
use futures::channel::oneshot;
use log::info;
use nym_authenticator_requests::latest::registration::{
    RemainingBandwidthData, BANDWIDTH_CAP_PER_DAY,
};
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth,
};
use nym_gateway_storage::GatewayStorage;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::WgApiWrapper;
use crate::{error::Error, peer_handle::SharedBandwidthStorageManager};
use crate::{peer_handle::PeerHandle, peer_storage_manager::PeerStorageManager};

pub enum PeerControlRequest {
    AddPeer {
        peer: Peer,
        client_id: Option<i64>,
        response_tx: oneshot::Sender<AddPeerControlResponse>,
    },
    RemovePeer {
        key: Key,
        response_tx: oneshot::Sender<RemovePeerControlResponse>,
    },
    QueryPeer {
        key: Key,
        response_tx: oneshot::Sender<QueryPeerControlResponse>,
    },
    QueryBandwidth {
        key: Key,
        response_tx: oneshot::Sender<QueryBandwidthControlResponse>,
    },
}

pub struct AddPeerControlResponse {
    pub success: bool,
}

pub struct RemovePeerControlResponse {
    pub success: bool,
}

pub struct QueryPeerControlResponse {
    pub success: bool,
    pub peer: Option<Peer>,
}

pub struct QueryBandwidthControlResponse {
    pub success: bool,
    pub bandwidth_data: Option<RemainingBandwidthData>,
}

pub struct PeerController {
    storage: GatewayStorage,
    // used to receive commands from individual handles too
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
    wg_api: Arc<WgApiWrapper>,
    host_information: Arc<RwLock<Host>>,
    bw_storage_managers: HashMap<Key, Option<SharedBandwidthStorageManager>>,
    timeout_check_interval: IntervalStream,
    task_client: nym_task::TaskClient,
}

impl PeerController {
    pub fn new(
        storage: GatewayStorage,
        wg_api: Arc<WgApiWrapper>,
        initial_host_information: Host,
        bw_storage_managers: HashMap<Key, (Option<SharedBandwidthStorageManager>, Peer)>,
        request_tx: mpsc::Sender<PeerControlRequest>,
        request_rx: mpsc::Receiver<PeerControlRequest>,
        task_client: nym_task::TaskClient,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let host_information = Arc::new(RwLock::new(initial_host_information));
        for (public_key, (bandwidth_storage_manager, peer)) in bw_storage_managers.iter() {
            let peer_storage_manager = PeerStorageManager::new(
                storage.clone(),
                peer.clone(),
                bandwidth_storage_manager.is_some(),
            );
            let mut handle = PeerHandle::new(
                public_key.clone(),
                host_information.clone(),
                peer_storage_manager,
                bandwidth_storage_manager.clone(),
                request_tx.clone(),
                &task_client,
            );
            tokio::spawn(async move {
                if let Err(e) = handle.run().await {
                    log::error!("Peer handle shut down ungracefully - {e}");
                }
            });
        }
        let bw_storage_managers = bw_storage_managers
            .into_iter()
            .map(|(k, (m, _))| (k, m))
            .collect();

        PeerController {
            storage,
            wg_api,
            host_information,
            bw_storage_managers,
            request_tx,
            request_rx,
            timeout_check_interval,
            task_client,
        }
    }

    // Function that should be used for peer insertion, to handle both storage and kernel interaction
    pub async fn add_peer(&self, peer: &Peer, client_id: Option<i64>) -> Result<(), Error> {
        if client_id.is_none() {
            self.storage.insert_wireguard_peer(peer, false).await?;
        }
        let ret: Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> =
            self.wg_api.inner.configure_peer(peer);
        if client_id.is_none() && ret.is_err() {
            // Try to revert the insertion in storage
            if self
                .storage
                .remove_wireguard_peer(&peer.public_key.to_string())
                .await
                .is_err()
            {
                log::error!("The storage has been corrupted. Wireguard peer {} will persist in storage indefinitely.", peer.public_key);
            }
        }
        Ok(ret?)
    }

    // Function that should be used for peer removal, to handle both storage and kernel interaction
    pub async fn remove_peer(&mut self, key: &Key) -> Result<(), Error> {
        self.storage.remove_wireguard_peer(&key.to_string()).await?;
        self.bw_storage_managers.remove(key);
        let ret = self.wg_api.inner.remove_peer(key);
        if ret.is_err() {
            log::error!("Wireguard peer could not be removed from wireguard kernel module. Process should be restarted so that the interface is reset.");
        }
        Ok(ret?)
    }

    pub async fn generate_bandwidth_manager(
        storage: GatewayStorage,
        public_key: &Key,
    ) -> Result<Option<BandwidthStorageManager>, Error> {
        if let Some(client_id) = storage
            .get_wireguard_peer(&public_key.to_string())
            .await?
            .ok_or(Error::MissingClientBandwidthEntry)?
            .client_id
        {
            let bandwidth = storage
                .get_available_bandwidth(client_id)
                .await?
                .ok_or(Error::MissingClientBandwidthEntry)?;
            Ok(Some(BandwidthStorageManager::new(
                storage,
                ClientBandwidth::new(bandwidth.into()),
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            )))
        } else {
            Ok(None)
        }
    }

    async fn handle_add_request(
        &mut self,
        peer: &Peer,
        client_id: Option<i64>,
    ) -> Result<(), Error> {
        self.add_peer(peer, client_id).await?;
        let bandwidth_storage_manager =
            Self::generate_bandwidth_manager(self.storage.clone(), &peer.public_key)
                .await?
                .map(|bw_m| Arc::new(RwLock::new(bw_m)));
        let peer_storage_manager = PeerStorageManager::new(
            self.storage.clone(),
            peer.clone(),
            bandwidth_storage_manager.is_some(),
        );
        let mut handle = PeerHandle::new(
            peer.public_key.clone(),
            self.host_information.clone(),
            peer_storage_manager,
            bandwidth_storage_manager.clone(),
            self.request_tx.clone(),
            &self.task_client,
        );
        self.bw_storage_managers
            .insert(peer.public_key.clone(), bandwidth_storage_manager);
        // try to immediately update the host information, to eliminate races
        if let Ok(host_information) = self.wg_api.inner.read_interface_data() {
            *self.host_information.write().await = host_information;
        }
        tokio::spawn(async move {
            if let Err(e) = handle.run().await {
                log::error!("Peer handle shut down ungracefully - {e}");
            }
        });
        Ok(())
    }

    async fn handle_query_peer(&self, key: &Key) -> Result<Option<Peer>, Error> {
        Ok(self
            .storage
            .get_wireguard_peer(&key.to_string())
            .await?
            .map(Peer::try_from)
            .transpose()?)
    }

    async fn handle_query_bandwidth(
        &self,
        key: &Key,
    ) -> Result<Option<RemainingBandwidthData>, Error> {
        let Some(bandwidth_storage_manager) = self.bw_storage_managers.get(key) else {
            return Ok(None);
        };
        let available_bandwidth = if let Some(bandwidth_storage_manager) = bandwidth_storage_manager
        {
            bandwidth_storage_manager
                .read()
                .await
                .available_bandwidth()
                .await
        } else {
            let Some(peer) = self.host_information.read().await.peers.get(key).cloned() else {
                // host information not updated yet
                return Ok(None);
            };
            BANDWIDTH_CAP_PER_DAY.saturating_sub(peer.rx_bytes + peer.tx_bytes) as i64
        };

        Ok(Some(RemainingBandwidthData {
            available_bandwidth,
        }))
    }

    pub async fn run(&mut self) {
        info!("started wireguard peer controller");
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    let Ok(host) = self.wg_api.inner.read_interface_data() else {
                        log::error!("Can't read wireguard kernel data");
                        continue;
                    };
                    let peers = host.peers.len();
                    let total_rx = host.peers.values().fold(0, |acc, peer| acc + peer.rx_bytes);
                    let total_tx = host.peers.values().fold(0, |acc, peer| acc + peer.tx_bytes);

                    println!("peers: {peers}, ↑↓ total_rx: {total_rx}, total_tx: {total_tx}");


                    *self.host_information.write().await = host;
                }
                _ = self.task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.request_rx.recv() => {
                    match msg {
                        Some(PeerControlRequest::AddPeer { peer, client_id, response_tx }) => {
                            let ret = self.handle_add_request(&peer, client_id).await;
                            if ret.is_ok() {
                                response_tx.send(AddPeerControlResponse { success: true }).ok();
                            } else {
                                response_tx.send(AddPeerControlResponse { success: false }).ok();
                            }
                        }
                        Some(PeerControlRequest::RemovePeer { key, response_tx }) => {
                            let success = self.remove_peer(&key).await.is_ok();
                            response_tx.send(RemovePeerControlResponse { success }).ok();
                        }
                        Some(PeerControlRequest::QueryPeer { key, response_tx }) => {
                            let ret = self.handle_query_peer(&key).await;
                            if let Ok(peer) = ret {
                                response_tx.send(QueryPeerControlResponse { success: true, peer }).ok();
                            } else {
                                response_tx.send(QueryPeerControlResponse { success: false, peer: None }).ok();
                            }
                        }
                        Some(PeerControlRequest::QueryBandwidth { key, response_tx }) => {
                            let ret = self.handle_query_bandwidth(&key).await;
                            if let Ok(bandwidth_data) = ret {
                                response_tx.send(QueryBandwidthControlResponse { success: true, bandwidth_data }).ok();
                            } else {
                                response_tx.send(QueryBandwidthControlResponse { success: false, bandwidth_data: None }).ok();
                            }
                        }
                        None => {
                            log::trace!("PeerController [main loop]: stopping since channel closed");
                            break;

                        }
                    }
                }
            }
        }
    }
}
