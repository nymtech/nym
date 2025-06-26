// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{
    host::{Host, Peer},
    key::Key,
    WireguardInterfaceApi,
};
use futures::channel::oneshot;
use log::info;
use nym_credential_verification::{
    bandwidth_storage_manager::BandwidthStorageManager, BandwidthFlushingBehaviourConfig,
    ClientBandwidth,
};
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_node_metrics::NymNodeMetrics;
use nym_wireguard_types::DEFAULT_PEER_TIMEOUT_CHECK;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::{error::Error, peer_handle::SharedBandwidthStorageManager};
use crate::{peer_handle::PeerHandle, peer_storage_manager::CachedPeerManager};

pub enum PeerControlRequest {
    AddPeer {
        peer: Peer,
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
    GetClientBandwidth {
        key: Key,
        response_tx: oneshot::Sender<GetClientBandwidthControlResponse>,
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

pub struct GetClientBandwidthControlResponse {
    pub client_bandwidth: Option<ClientBandwidth>,
}

pub struct PeerController {
    storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,

    // we have "all" metrics of a node, but they're behind a single Arc pointer,
    // so the overhead is minimal
    metrics: NymNodeMetrics,

    // used to receive commands from individual handles too
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
    wg_api: Arc<dyn WireguardInterfaceApi + Send + Sync>,
    host_information: Arc<RwLock<Host>>,
    bw_storage_managers: HashMap<Key, SharedBandwidthStorageManager>,
    timeout_check_interval: IntervalStream,
    task_client: nym_task::TaskClient,
}

impl PeerController {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,
        metrics: NymNodeMetrics,
        wg_api: Arc<dyn WireguardInterfaceApi + Send + Sync>,
        initial_host_information: Host,
        bw_storage_managers: HashMap<Key, (SharedBandwidthStorageManager, Peer)>,
        request_tx: mpsc::Sender<PeerControlRequest>,
        request_rx: mpsc::Receiver<PeerControlRequest>,
        task_client: nym_task::TaskClient,
    ) -> Self {
        let timeout_check_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK),
        );
        let host_information = Arc::new(RwLock::new(initial_host_information));
        for (public_key, (bandwidth_storage_manager, peer)) in bw_storage_managers.iter() {
            let cached_peer_manager = CachedPeerManager::new(peer);
            let mut handle = PeerHandle::new(
                public_key.clone(),
                host_information.clone(),
                cached_peer_manager,
                bandwidth_storage_manager.clone(),
                request_tx.clone(),
                &task_client,
            );
            let public_key = public_key.clone();
            tokio::spawn(async move {
                handle.run().await;
                log::debug!("Peer handle shut down for {public_key}");
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
            metrics,
        }
    }

    // Function that should be used for peer removal, to handle both storage and kernel interaction
    pub async fn remove_peer(&mut self, key: &Key) -> Result<(), Error> {
        self.storage.remove_wireguard_peer(&key.to_string()).await?;
        self.bw_storage_managers.remove(key);
        let ret = self.wg_api.remove_peer(key);
        if ret.is_err() {
            log::error!("Wireguard peer could not be removed from wireguard kernel module. Process should be restarted so that the interface is reset.");
        }
        Ok(ret?)
    }

    pub async fn generate_bandwidth_manager(
        storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,
        public_key: &Key,
    ) -> Result<BandwidthStorageManager, Error> {
        let client_id = storage
            .get_wireguard_peer(&public_key.to_string())
            .await?
            .ok_or(Error::MissingClientBandwidthEntry)?
            .client_id;

        let bandwidth = storage
            .get_available_bandwidth(client_id)
            .await?
            .ok_or(Error::MissingClientBandwidthEntry)?;

        Ok(BandwidthStorageManager::new(
            storage,
            ClientBandwidth::new(bandwidth.into()),
            client_id,
            BandwidthFlushingBehaviourConfig::default(),
            true,
        ))
    }

    async fn handle_add_request(&mut self, peer: &Peer) -> Result<(), Error> {
        self.wg_api.configure_peer(peer)?;
        let bandwidth_storage_manager = Arc::new(RwLock::new(
            Self::generate_bandwidth_manager(
                dyn_clone::clone_box(&*self.storage),
                &peer.public_key,
            )
            .await?,
        ));
        let cached_peer_manager = CachedPeerManager::new(peer);
        let mut handle = PeerHandle::new(
            peer.public_key.clone(),
            self.host_information.clone(),
            cached_peer_manager,
            bandwidth_storage_manager.clone(),
            self.request_tx.clone(),
            &self.task_client,
        );
        self.bw_storage_managers
            .insert(peer.public_key.clone(), bandwidth_storage_manager);
        // try to immediately update the host information, to eliminate races
        if let Ok(host_information) = self.wg_api.read_interface_data() {
            *self.host_information.write().await = host_information;
        }
        let public_key = peer.public_key.clone();
        tokio::spawn(async move {
            handle.run().await;
            log::debug!("Peer handle shut down for {public_key}");
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

    async fn handle_get_client_bandwidth(&self, key: &Key) -> Option<ClientBandwidth> {
        if let Some(bandwidth_storage_manager) = self.bw_storage_managers.get(key) {
            Some(bandwidth_storage_manager.read().await.client_bandwidth())
        } else {
            None
        }
    }

    async fn update_metrics(&self, new_host: &Host) {
        let now = SystemTime::now();
        const ACTIVITY_THRESHOLD: Duration = Duration::from_secs(180);

        let old_host = self.host_information.read().await;

        let total_peers = new_host.peers.len();
        let mut active_peers = 0;
        let mut new_rx = 0;
        let mut new_tx = 0;

        for (peer_key, peer) in new_host.peers.iter() {
            match old_host.peers.get(peer_key) {
                // only consider pre-existing peers for the purposes of bandwidth accounting,
                // so that the value would always be increasing.
                Some(prior) => {
                    // 1. determine bandwidth changes
                    let delta_rx = peer.rx_bytes.saturating_sub(prior.rx_bytes);
                    let delta_tx = peer.tx_bytes.saturating_sub(prior.tx_bytes);

                    new_rx += delta_rx;
                    new_tx += delta_tx;

                    // 2. attempt to determine if the peer is still active

                    // 2.1. if there were bytes sent and received on the link since last it was called,
                    // the peer is definitely still active
                    if delta_rx > 0 && delta_tx > 0 {
                        active_peers += 1;
                        continue;
                    }

                    // 2.2. otherwise attempt to look at time since last handshake -
                    // if no handshake occurred in the last 3min, we assume the connection might be dead
                    let Some(last_handshake) = peer.last_handshake else {
                        continue;
                    };
                    let Ok(elapsed) = now.duration_since(last_handshake) else {
                        continue;
                    };
                    if elapsed < ACTIVITY_THRESHOLD {
                        active_peers += 1;
                    }
                }
                None => {
                    // if it's a brand-new peer, and it hasn't repeated the handshake in the last 3 min,
                    // we assume the connection might be dead
                    let Some(last_handshake) = peer.last_handshake else {
                        continue;
                    };
                    let Ok(elapsed) = now.duration_since(last_handshake) else {
                        continue;
                    };
                    if elapsed < ACTIVITY_THRESHOLD {
                        active_peers += 1;
                    }
                }
            }
        }

        self.metrics.wireguard.update(
            // if the conversion fails it means we're running not running on a 64bit system
            // and that's a reason enough for this failure.
            new_rx.try_into().expect(
                "failed to convert bytes from u64 to usize - are you running on non 64bit system?",
            ),
            new_tx.try_into().expect(
                "failed to convert bytes from u64 to usize - are you running on non 64bit system?",
            ),
            total_peers,
            active_peers,
        );
    }

    pub async fn run(&mut self) {
        info!("started wireguard peer controller");
        loop {
            tokio::select! {
                _ = self.timeout_check_interval.next() => {
                    let Ok(host) = self.wg_api.read_interface_data() else {
                        log::error!("Can't read wireguard kernel data");
                        continue;
                    };
                    self.update_metrics(&host).await;

                    *self.host_information.write().await = host;
                }
                _ = self.task_client.recv() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.request_rx.recv() => {
                    match msg {
                        Some(PeerControlRequest::AddPeer { peer, response_tx }) => {
                            let ret = self.handle_add_request(&peer).await;
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
                        Some(PeerControlRequest::GetClientBandwidth { key, response_tx }) => {
                            let client_bandwidth = self.handle_get_client_bandwidth(&key).await;
                            response_tx.send(GetClientBandwidthControlResponse { client_bandwidth }).ok();
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

#[cfg(feature = "mock")]
#[derive(Default)]
struct MockWgApi {
    peers: std::sync::RwLock<HashMap<Key, Peer>>,
}

#[cfg(feature = "mock")]
impl WireguardInterfaceApi for MockWgApi {
    fn create_interface(
        &self,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn assign_address(
        &self,
        _address: &defguard_wireguard_rs::net::IpAddrMask,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn configure_peer_routing(
        &self,
        _peers: &[Peer],
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    #[cfg(not(target_os = "windows"))]
    fn configure_interface(
        &self,
        _config: &defguard_wireguard_rs::InterfaceConfiguration,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    #[cfg(target_os = "windows")]
    fn configure_interface(
        &self,
        _config: &defguard_wireguard_rs::InterfaceConfiguration,
        _dns: &[std::net::IpAddr],
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn remove_interface(
        &self,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn configure_peer(
        &self,
        peer: &Peer,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.peers
            .write()
            .unwrap()
            .insert(peer.public_key.clone(), peer.clone());
        Ok(())
    }

    fn remove_peer(
        &self,
        peer_pubkey: &Key,
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.peers.write().unwrap().remove(peer_pubkey);
        Ok(())
    }

    fn read_interface_data(
        &self,
    ) -> Result<Host, defguard_wireguard_rs::error::WireguardInterfaceError> {
        let mut host = Host::default();
        host.peers = self.peers.read().unwrap().clone();
        Ok(host)
    }

    fn configure_dns(
        &self,
        _dns: &[std::net::IpAddr],
    ) -> Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }
}

#[cfg(feature = "mock")]
pub fn start_controller(
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
) -> (
    nym_gateway_storage::traits::mock::MockGatewayStorage,
    nym_task::TaskManager,
) {
    let storage = nym_gateway_storage::traits::mock::MockGatewayStorage::default();
    let wg_api = Arc::new(MockWgApi::default());
    let task_manager = nym_task::TaskManager::default();
    let mut peer_controller = PeerController::new(
        Box::new(storage.clone()),
        Default::default(),
        wg_api,
        Default::default(),
        Default::default(),
        request_tx,
        request_rx,
        task_manager.subscribe(),
    );
    tokio::spawn(async move { peer_controller.run().await });

    (storage, task_manager)
}

#[cfg(feature = "mock")]
pub async fn stop_controller(mut task_manager: nym_task::TaskManager) {
    task_manager.signal_shutdown().unwrap();
    task_manager.wait_for_shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_and_stop() {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (_, task_manager) = start_controller(request_tx.clone(), request_rx);
        stop_controller(task_manager).await;
    }

    // #[tokio::test]
    // async fn add_peer() {
    //     let (request_tx, storage, mut task_manager) = start_controller();
    //     let peer = Peer::default();

    //     let (response_tx, response_rx) = oneshot::channel();
    //     request_tx
    //         .send(PeerControlRequest::AddPeer {
    //             peer: peer.clone(),
    //             response_tx,
    //         })
    //         .await
    //         .unwrap();
    //     let response = response_rx.await.unwrap();
    //     assert!(!response.success);

    //     storage
    //         .insert_wireguard_peer(&peer, FromStr::from_str("entry_wireguard").unwrap())
    //         .await
    //         .unwrap();
    //     let (response_tx, response_rx) = oneshot::channel();
    //     request_tx
    //         .send(PeerControlRequest::AddPeer { peer, response_tx })
    //         .await
    //         .unwrap();
    //     let response = response_rx.await.unwrap();
    //     assert!(response.success);

    //     task_manager.signal_shutdown().unwrap();
    //     task_manager.wait_for_shutdown().await;
    // }
}
