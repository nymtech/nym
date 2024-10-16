// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    WireguardInterfaceApi,
};
use futures::channel::oneshot;
use nym_authenticator_requests::{
    latest::registration::RemainingBandwidthData, v1::registration::BANDWIDTH_CAP_PER_DAY,
};
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth,
};
use nym_gateway_storage::Storage;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::peer_handle::PeerHandle;
use crate::WgApiWrapper;
use crate::{error::Error, peer_handle::SharedBandwidthStorageManager};

pub enum PeerControlRequest {
    AddPeer {
        peer: Peer,
        ticket_validation: bool,
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
    pub client_id: Option<i64>,
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

pub struct PeerController<St: Storage + Clone + 'static> {
    storage: St,
    // used to receive commands from individual handles too
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
    wg_api: Arc<WgApiWrapper>,
    host_information: Arc<RwLock<Host>>,
    bw_storage_managers: HashMap<Key, Option<SharedBandwidthStorageManager<St>>>,
    timeout_check_interval: IntervalStream,
    task_client: nym_task::TaskClient,
}

impl<St: Storage + Clone + 'static> PeerController<St> {
    pub fn new(
        storage: St,
        wg_api: Arc<WgApiWrapper>,
        initial_host_information: Host,
        bw_storage_managers: HashMap<Key, Option<SharedBandwidthStorageManager<St>>>,
        request_tx: mpsc::Sender<PeerControlRequest>,
        request_rx: mpsc::Receiver<PeerControlRequest>,
        task_client: nym_task::TaskClient,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let host_information = Arc::new(RwLock::new(initial_host_information));
        for (public_key, bandwidth_storage_manager) in bw_storage_managers.iter() {
            let mut handle = PeerHandle::new(
                storage.clone(),
                public_key.clone(),
                host_information.clone(),
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
    pub async fn add_peer(&self, peer: &Peer, with_client_id: bool) -> Result<Option<i64>, Error> {
        let client_id = self
            .storage
            .insert_wireguard_peer(peer, with_client_id)
            .await?;
        let ret = self.wg_api.inner.configure_peer(peer);
        if ret.is_err() {
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
        ret?;
        Ok(client_id)
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
        storage: St,
        public_key: &Key,
    ) -> Result<Option<BandwidthStorageManager<St>>, Error> {
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
        with_client_id: bool,
    ) -> Result<Option<i64>, Error> {
        let client_id = self.add_peer(peer, with_client_id).await?;
        let bandwidth_storage_manager =
            Self::generate_bandwidth_manager(self.storage.clone(), &peer.public_key)
                .await?
                .map(|bw_m| Arc::new(RwLock::new(bw_m)));
        let mut handle = PeerHandle::new(
            self.storage.clone(),
            peer.public_key.clone(),
            self.host_information.clone(),
            bandwidth_storage_manager.clone(),
            self.request_tx.clone(),
            &self.task_client,
        );
        self.bw_storage_managers
            .insert(peer.public_key.clone(), bandwidth_storage_manager);
        tokio::spawn(async move {
            if let Err(e) = handle.run().await {
                log::error!("Peer handle shut down ungracefully - {e}");
            }
        });
        Ok(client_id)
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
            let peer = self
                .host_information
                .read()
                .await
                .peers
                .get(key)
                .ok_or(Error::PeerMismatch)?
                .clone();
            BANDWIDTH_CAP_PER_DAY.saturating_sub((peer.rx_bytes + peer.tx_bytes) as i64)
        };

        Ok(Some(RemainingBandwidthData {
            available_bandwidth,
        }))
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    let Ok(host) = self.wg_api.inner.read_interface_data() else {
                        log::error!("Can't read wireguard kernel data");
                        continue;
                    };
                    *self.host_information.write().await = host;
                }
                _ = self.task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.request_rx.recv() => {
                    match msg {
                        Some(PeerControlRequest::AddPeer { peer, ticket_validation, response_tx }) => {
                            let ret = self.handle_add_request(&peer, ticket_validation).await;
                            if let Ok(client_id) = ret {
                                response_tx.send(AddPeerControlResponse { success: true, client_id }).ok();
                            } else {
                                response_tx.send(AddPeerControlResponse { success: false, client_id: None }).ok();
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
