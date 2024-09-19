// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    WireguardInterfaceApi,
};
use futures::channel::oneshot;
use nym_authenticator_requests::v2::registration::{RemainingBandwidthData, BANDWIDTH_CAP_PER_DAY};
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth,
};
use nym_gateway_storage::Storage;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::error::Error;
use crate::peer_handle::PeerHandle;
use crate::WgApiWrapper;

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
    pub bandwidth_data: Option<RemainingBandwidthData>,
}

pub struct PeerController<St: Storage + Clone + 'static> {
    storage: St,
    // used to receive commands from individual handles too
    request_tx: mpsc::UnboundedSender<PeerControlRequest>,
    request_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
    wg_api: Arc<WgApiWrapper>,
    host_information: Arc<RwLock<Host>>,
    timeout_check_interval: IntervalStream,
    task_client: nym_task::TaskClient,
}

impl<St: Storage + Clone + 'static> PeerController<St> {
    pub async fn new(
        storage: St,
        wg_api: Arc<WgApiWrapper>,
        initial_host_information: Host,
        startup_peers: Vec<Peer>,
        request_tx: mpsc::UnboundedSender<PeerControlRequest>,
        request_rx: mpsc::UnboundedReceiver<PeerControlRequest>,
        task_client: nym_task::TaskClient,
    ) -> Result<Self, Error> {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let host_information = Arc::new(RwLock::new(initial_host_information));
        for peer in startup_peers {
            let bandwidth_storage_manager =
                Self::generate_bandwidth_manager(storage.clone(), &peer.public_key).await?;
            let mut handle = PeerHandle::new(
                storage.clone(),
                peer.public_key.clone(),
                host_information.clone(),
                bandwidth_storage_manager,
                request_tx.clone(),
                &task_client,
            );
            tokio::spawn(async move {
                if let Err(e) = handle.run().await {
                    log::error!("Peer handle shut down ungracefully - {e}");
                }
            });
        }

        Ok(PeerController {
            storage,
            wg_api,
            host_information,
            request_tx,
            request_rx,
            timeout_check_interval,
            task_client,
        })
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
    pub async fn remove_peer(&self, key: &Key) -> Result<(), Error> {
        self.storage.remove_wireguard_peer(&key.to_string()).await?;
        let ret = self.wg_api.inner.remove_peer(key);
        if ret.is_err() {
            log::error!("Wireguard peer could not be removed from wireguard kernel module. Process should be restarted so that the interface is reset.");
        }
        Ok(ret?)
    }

    async fn generate_bandwidth_manager(
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
        &self,
        peer: &Peer,
        with_client_id: bool,
    ) -> Result<Option<i64>, Error> {
        let client_id = self.add_peer(peer, with_client_id).await?;
        let bandwidth_storage_manager =
            Self::generate_bandwidth_manager(self.storage.clone(), &peer.public_key).await?;
        let mut handle = PeerHandle::new(
            self.storage.clone(),
            peer.public_key.clone(),
            self.host_information.clone(),
            bandwidth_storage_manager,
            self.request_tx.clone(),
            &self.task_client,
        );
        tokio::spawn(async move {
            if let Err(e) = handle.run().await {
                log::error!("Peer handle shut down ungracefully - {e}");
            }
        });
        Ok(client_id)
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
                        Some(PeerControlRequest::QueryPeer{key,response_tx}) => {
                            let (success, peer) = match self.storage.get_wireguard_peer(&key.to_string()).await {
                                Err(e) => {
                                    log::error!("Could not query peer storage {e}");
                                    (false, None)
                                },
                                Ok(None) => (true, None),
                                Ok(Some(storage_peer)) => {
                                    match Peer::try_from(storage_peer) {
                                        Ok(peer) => (true, Some(peer)),
                                        Err(e) => {
                                            log::error!("Could not parse storage peer {e}");
                                            (false, None)
                                        }
                                    }
                                },
                            };
                            response_tx.send(QueryPeerControlResponse { success, peer }).ok();
                        }
                        Some(PeerControlRequest::QueryBandwidth{key, response_tx}) => {
                            // let msg = if self.suspended_peers.contains_key(&key) {
                            //     PeerControlResponse::QueryBandwidth { bandwidth_data: Some(RemainingBandwidthData{ available_bandwidth: 0, suspended: true }) }
                            // } else if let Some(&consumed_bandwidth) = self.last_seen_bandwidth.get(&key) {
                            //     PeerControlResponse::QueryBandwidth { bandwidth_data: Some(RemainingBandwidthData{ available_bandwidth: BANDWIDTH_CAP_PER_DAY - consumed_bandwidth, suspended: false })}
                            // } else {
                            //     PeerControlResponse::QueryBandwidth { bandwidth_data: None }
                            // };
                            // response_tx.send(msg).ok();
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
