// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{
    WireguardInterfaceApi,
    host::{Host, Peer},
    key::Key,
};
use futures::channel::oneshot;
use log::info;
use nym_credential_verification::{
    BandwidthFlushingBehaviourConfig, ClientBandwidth, CredentialVerifier, TicketVerifier,
    bandwidth_storage_manager::BandwidthStorageManager, ecash::traits::EcashManager,
};
use nym_credentials_interface::CredentialSpendingData;
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_ip_packet_requests::IpPair;
use nym_node_metrics::NymNodeMetrics;
use nym_wireguard_types::{
    DEFAULT_IP_CLEANUP_INTERVAL, DEFAULT_IP_STALE_AGE, DEFAULT_PEER_TIMEOUT_CHECK,
};
use std::{collections::HashMap, sync::Arc};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, SystemTime},
};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::{StreamExt, wrappers::IntervalStream};

use crate::{
    error::{Error, Result},
    ip_pool::IpPool,
    peer_handle::SharedBandwidthStorageManager,
};
use crate::{peer_handle::PeerHandle, peer_storage_manager::CachedPeerManager};

/// Registration data for a new peer (without pre-allocated IPs)
#[derive(Debug, Clone)]
pub struct PeerRegistrationData {
    pub public_key: Key,
    pub preshared_key: Option<Key>,
    pub endpoint: Option<SocketAddr>,
    pub persistent_keepalive_interval: Option<u16>,
}

impl PeerRegistrationData {
    pub fn new(public_key: Key) -> Self {
        Self {
            public_key,
            preshared_key: None,
            endpoint: None,
            persistent_keepalive_interval: None,
        }
    }

    pub fn with_preshared_key(mut self, key: Key) -> Self {
        self.preshared_key = Some(key);
        self
    }

    pub fn with_endpoint(mut self, endpoint: SocketAddr) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_keepalive(mut self, interval: u16) -> Self {
        self.persistent_keepalive_interval = Some(interval);
        self
    }
}

pub enum PeerControlRequest {
    /// Add a peer with pre-allocated IPs (for backwards compatibility)
    AddPeer {
        peer: Peer,
        response_tx: oneshot::Sender<AddPeerControlResponse>,
    },
    /// Register a new peer and allocate IPs from the pool
    RegisterPeer {
        registration_data: PeerRegistrationData,
        response_tx: oneshot::Sender<RegisterPeerControlResponse>,
    },
    RemovePeer {
        key: Key,
        response_tx: oneshot::Sender<RemovePeerControlResponse>,
    },
    QueryPeer {
        key: Key,
        response_tx: oneshot::Sender<QueryPeerControlResponse>,
    },
    GetClientBandwidthByKey {
        key: Key,
        response_tx: oneshot::Sender<GetClientBandwidthControlResponse>,
    },
    GetClientBandwidthByIp {
        ip: IpAddr,
        response_tx: oneshot::Sender<GetClientBandwidthControlResponse>,
    },
    GetVerifierByKey {
        key: Key,
        credential: Box<CredentialSpendingData>,
        response_tx: oneshot::Sender<QueryVerifierControlResponse>,
    },
    GetVerifierByIp {
        ip: IpAddr,
        credential: Box<CredentialSpendingData>,
        response_tx: oneshot::Sender<QueryVerifierControlResponse>,
    },
}

pub type AddPeerControlResponse = Result<()>;
pub type RegisterPeerControlResponse = Result<IpPair>;
pub type RemovePeerControlResponse = Result<()>;
pub type QueryPeerControlResponse = Result<Option<Peer>>;
pub type GetClientBandwidthControlResponse = Result<ClientBandwidth>;
pub type QueryVerifierControlResponse = Result<Box<dyn TicketVerifier + Send + Sync>>;

pub struct PeerController {
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,

    // we have "all" metrics of a node, but they're behind a single Arc pointer,
    // so the overhead is minimal
    metrics: NymNodeMetrics,

    // IP address pool for peer allocation
    ip_pool: IpPool,

    // used to receive commands from individual handles too
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
    wg_api: Arc<dyn WireguardInterfaceApi + Send + Sync>,
    host_information: Arc<RwLock<Host>>,
    bw_storage_managers: HashMap<Key, SharedBandwidthStorageManager>,
    timeout_check_interval: IntervalStream,
    ip_cleanup_interval: IntervalStream,
    shutdown_token: nym_task::ShutdownToken,
}

impl PeerController {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
        metrics: NymNodeMetrics,
        ip_pool: IpPool,
        wg_api: Arc<dyn WireguardInterfaceApi + Send + Sync>,
        initial_host_information: Host,
        bw_storage_managers: HashMap<Key, (SharedBandwidthStorageManager, Peer)>,
        request_tx: mpsc::Sender<PeerControlRequest>,
        request_rx: mpsc::Receiver<PeerControlRequest>,
        shutdown_token: nym_task::ShutdownToken,
    ) -> Self {
        let timeout_check_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_PEER_TIMEOUT_CHECK));
        let ip_cleanup_interval =
            IntervalStream::new(tokio::time::interval(DEFAULT_IP_CLEANUP_INTERVAL));
        let host_information = Arc::new(RwLock::new(initial_host_information));
        for (public_key, (bandwidth_storage_manager, peer)) in bw_storage_managers.iter() {
            let cached_peer_manager = CachedPeerManager::new(peer);
            let mut handle = PeerHandle::new(
                public_key.clone(),
                host_information.clone(),
                cached_peer_manager,
                bandwidth_storage_manager.clone(),
                request_tx.clone(),
                &shutdown_token,
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
            ecash_verifier,
            metrics,
            ip_pool,
            wg_api,
            host_information,
            bw_storage_managers,
            request_tx,
            request_rx,
            timeout_check_interval,
            ip_cleanup_interval,
            shutdown_token,
        }
    }

    // Function that should be used for peer removal, to handle both storage and kernel interaction
    pub async fn remove_peer(&mut self, key: &Key) -> Result<()> {
        nym_metrics::inc!("wg_peer_removal_attempts");

        self.ecash_verifier
            .storage()
            .remove_wireguard_peer(&key.to_string())
            .await?;
        self.bw_storage_managers.remove(key);
        let ret = self.wg_api.remove_peer(key);
        if ret.is_err() {
            nym_metrics::inc!("wg_peer_removal_failed");
            log::error!(
                "Wireguard peer could not be removed from wireguard kernel module. Process should be restarted so that the interface is reset."
            );
        } else {
            nym_metrics::inc!("wg_peer_removal_success");
        }
        Ok(ret?)
    }

    pub async fn generate_bandwidth_manager(
        storage: Box<dyn BandwidthGatewayStorage + Send + Sync>,
        public_key: &Key,
    ) -> Result<BandwidthStorageManager> {
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

    async fn handle_add_request(&mut self, peer: &Peer) -> Result<()> {
        nym_metrics::inc!("wg_peer_addition_attempts");

        // Try to configure WireGuard peer
        if let Err(e) = self.wg_api.configure_peer(peer) {
            nym_metrics::inc!("wg_peer_addition_failed");
            nym_metrics::inc!("wg_config_errors_total");
            return Err(e.into());
        }

        let bandwidth_storage_manager = SharedBandwidthStorageManager::new(
            Arc::new(RwLock::new(
                Self::generate_bandwidth_manager(self.ecash_verifier.storage(), &peer.public_key)
                    .await?,
            )),
            peer.allowed_ips.clone(),
        );
        let cached_peer_manager = CachedPeerManager::new(peer);
        let mut handle = PeerHandle::new(
            peer.public_key.clone(),
            self.host_information.clone(),
            cached_peer_manager,
            bandwidth_storage_manager.clone(),
            self.request_tx.clone(),
            &self.shutdown_token,
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

        nym_metrics::inc!("wg_peer_addition_success");
        Ok(())
    }

    /// Allocate IP pair from pool for a new peer registration
    ///
    /// This only allocates IPs - the caller must handle database storage and
    /// then call AddPeer with a complete Peer struct.
    async fn handle_register_request(
        &mut self,
        _registration_data: PeerRegistrationData,
    ) -> Result<IpPair> {
        nym_metrics::inc!("wg_ip_allocation_attempts");

        // Allocate IP pair from pool
        let ip_pair = self
            .ip_pool
            .allocate()
            .await
            .map_err(|e| Error::IpPool(e.to_string()))?;

        nym_metrics::inc!("wg_ip_allocation_success");
        tracing::debug!("Allocated IP pair: {}", ip_pair);

        Ok(ip_pair)
    }

    async fn ip_to_key(&self, ip: IpAddr) -> Result<Option<Key>> {
        Ok(self
            .bw_storage_managers
            .iter()
            .find_map(|(key, bw_manager)| {
                bw_manager
                    .allowed_ips()
                    .iter()
                    .find(|ip_mask| ip_mask.ip == ip)
                    .and(Some(key.clone()))
            }))
    }

    async fn handle_query_peer_by_key(&self, key: &Key) -> Result<Option<Peer>> {
        Ok(self
            .ecash_verifier
            .storage()
            .get_wireguard_peer(&key.to_string())
            .await?
            .map(Peer::try_from)
            .transpose()?)
    }

    async fn handle_get_client_bandwidth_by_key(&self, key: &Key) -> Result<ClientBandwidth> {
        let bandwidth_storage_manager = self
            .bw_storage_managers
            .get(key)
            .ok_or(Error::MissingClientBandwidthEntry)?;

        Ok(bandwidth_storage_manager
            .inner()
            .read()
            .await
            .client_bandwidth())
    }

    async fn handle_get_client_bandwidth_by_ip(&self, ip: IpAddr) -> Result<ClientBandwidth> {
        let Some(key) = self.ip_to_key(ip).await? else {
            return Err(Error::MissingClientKernelEntry(ip.to_string()));
        };

        self.handle_get_client_bandwidth_by_key(&key).await
    }

    async fn handle_query_verifier_by_key(
        &self,
        key: &Key,
        credential: CredentialSpendingData,
    ) -> Result<Box<dyn TicketVerifier + Send + Sync>> {
        let storage = self.ecash_verifier.storage();
        let client_id = storage
            .get_wireguard_peer(&key.to_string())
            .await?
            .ok_or(Error::MissingClientBandwidthEntry)?
            .client_id;
        let Some(bandwidth_storage_manager) = self.bw_storage_managers.get(key) else {
            return Err(Error::MissingClientBandwidthEntry);
        };
        let client_bandwidth = bandwidth_storage_manager
            .inner()
            .read()
            .await
            .client_bandwidth();
        let verifier = CredentialVerifier::new(
            CredentialSpendingRequest::new(credential),
            self.ecash_verifier.clone(),
            BandwidthStorageManager::new(
                storage,
                client_bandwidth,
                client_id,
                BandwidthFlushingBehaviourConfig::default(),
                true,
            ),
        );
        Ok(Box::new(verifier))
    }

    async fn handle_query_verifier_by_ip(
        &self,
        ip: IpAddr,
        credential: CredentialSpendingData,
    ) -> Result<Box<dyn TicketVerifier + Send + Sync>> {
        let Some(key) = self.ip_to_key(ip).await? else {
            return Err(Error::MissingClientKernelEntry(ip.to_string()));
        };

        self.handle_query_verifier_by_key(&key, credential).await
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
                _ = self.ip_cleanup_interval.next() => {
                    // Periodically cleanup stale IP allocations
                    let freed = self.ip_pool.cleanup_stale(DEFAULT_IP_STALE_AGE).await;
                    if freed > 0 {
                        nym_metrics::inc_by!("wg_stale_ips_cleaned", freed as u64);
                        log::info!("Cleaned up {} stale IP allocations", freed);
                    }
                }
                _ = self.shutdown_token.cancelled() => {
                    log::trace!("PeerController handler: Received shutdown");
                    break;
                }
                msg = self.request_rx.recv() => {
                    match msg {
                        Some(PeerControlRequest::AddPeer { peer, response_tx }) => {
                            response_tx.send(self.handle_add_request(&peer).await).ok();
                        }
                        Some(PeerControlRequest::RegisterPeer { registration_data, response_tx }) => {
                            response_tx.send(self.handle_register_request(registration_data).await).ok();
                        }
                        Some(PeerControlRequest::RemovePeer { key, response_tx }) => {
                            response_tx.send(self.remove_peer(&key).await).ok();
                        }
                        Some(PeerControlRequest::QueryPeer { key, response_tx }) => {
                            response_tx.send(self.handle_query_peer_by_key(&key).await).ok();
                        }
                        Some(PeerControlRequest::GetClientBandwidthByKey { key, response_tx }) => {
                            response_tx.send(self.handle_get_client_bandwidth_by_key(&key).await).ok();
                        }
                        Some(PeerControlRequest::GetClientBandwidthByIp { ip, response_tx }) => {
                            response_tx.send(self.handle_get_client_bandwidth_by_ip(ip).await).ok();
                        }
                        Some(PeerControlRequest::GetVerifierByKey { key, credential, response_tx }) => {
                            response_tx.send(self.handle_query_verifier_by_key(&key, *credential).await).ok();
                        }
                        Some(PeerControlRequest::GetVerifierByIp { ip, credential, response_tx }) => {
                            response_tx.send(self.handle_query_verifier_by_ip(ip, *credential).await).ok();
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
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn assign_address(
        &self,
        _address: &defguard_wireguard_rs::net::IpAddrMask,
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn configure_peer_routing(
        &self,
        _peers: &[Peer],
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    #[cfg(not(target_os = "windows"))]
    fn configure_interface(
        &self,
        _config: &defguard_wireguard_rs::InterfaceConfiguration,
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    #[cfg(target_os = "windows")]
    fn configure_interface(
        &self,
        _config: &defguard_wireguard_rs::InterfaceConfiguration,
        _dns: &[std::net::IpAddr],
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn remove_interface(
        &self,
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }

    fn configure_peer(
        &self,
        peer: &Peer,
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.peers
            .write()
            .unwrap()
            .insert(peer.public_key.clone(), peer.clone());
        Ok(())
    }

    fn remove_peer(
        &self,
        peer_pubkey: &Key,
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        self.peers.write().unwrap().remove(peer_pubkey);
        Ok(())
    }

    fn read_interface_data(
        &self,
    ) -> std::result::Result<Host, defguard_wireguard_rs::error::WireguardInterfaceError> {
        let mut host = Host::default();
        host.peers = self.peers.read().unwrap().clone();
        Ok(host)
    }

    fn configure_dns(
        &self,
        _dns: &[std::net::IpAddr],
    ) -> std::result::Result<(), defguard_wireguard_rs::error::WireguardInterfaceError> {
        todo!()
    }
}

#[cfg(feature = "mock")]
pub fn start_controller(
    request_tx: mpsc::Sender<PeerControlRequest>,
    request_rx: mpsc::Receiver<PeerControlRequest>,
) -> (
    Arc<RwLock<nym_gateway_storage::traits::mock::MockGatewayStorage>>,
    nym_task::ShutdownManager,
) {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Arc;

    let storage = Arc::new(RwLock::new(
        nym_gateway_storage::traits::mock::MockGatewayStorage::default(),
    ));
    let ecash_manager = Arc::new(nym_credential_verification::ecash::MockEcashManager::new(
        Box::new(storage.clone()),
    ));
    let wg_api = Arc::new(MockWgApi::default());

    // Create IP pool for testing
    let ip_pool = IpPool::new(
        Ipv4Addr::new(10, 0, 0, 0),
        24,
        Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 0),
        112,
    )
    .expect("Failed to create IP pool for testing");

    let shutdown_manager = nym_task::ShutdownManager::empty_mock();
    let mut peer_controller = PeerController::new(
        ecash_manager,
        Default::default(),
        ip_pool,
        wg_api,
        Default::default(),
        Default::default(),
        request_tx,
        request_rx,
        shutdown_manager.child_shutdown_token(),
    );
    tokio::spawn(async move { peer_controller.run().await });

    (storage, shutdown_manager)
}

#[cfg(feature = "mock")]
pub async fn stop_controller(mut shutdown_manager: nym_task::ShutdownManager) {
    shutdown_manager.send_cancellation();
    shutdown_manager.run_until_shutdown().await;
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_and_stop() {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (_, shutdown_manager) = start_controller(request_tx.clone(), request_rx);
        stop_controller(shutdown_manager).await;
    }
}
