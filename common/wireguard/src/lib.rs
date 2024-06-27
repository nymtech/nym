#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

use dashmap::DashMap;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, WGApi};
use nym_crypto::asymmetric::encryption::KeyPair;
use nym_wireguard_types::{Config, Error, GatewayClient, GatewayClientRegistry};
use peer_controller::PeerControlMessage;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver};

const WG_TUN_NAME: &str = "nymwg";

pub mod peer_controller;

pub struct WgApiWrapper {
    inner: WGApi,
}

impl WgApiWrapper {
    pub fn new(wg_api: WGApi) -> Self {
        WgApiWrapper { inner: wg_api }
    }
}

impl Drop for WgApiWrapper {
    fn drop(&mut self) {
        if let Err(e) = defguard_wireguard_rs::WireguardInterfaceApi::remove_interface(&self.inner)
        {
            log::error!("Could not remove the wireguard interface: {:?}", e);
        }
    }
}

#[derive(Clone)]
pub struct WireguardGatewayData {
    config: Config,
    keypair: Arc<KeyPair>,
    client_registry: Arc<GatewayClientRegistry>,
    peer_tx: mpsc::UnboundedSender<PeerControlMessage>,
}

impl WireguardGatewayData {
    pub fn new(
        config: Config,
        keypair: Arc<KeyPair>,
    ) -> (Self, mpsc::UnboundedReceiver<PeerControlMessage>) {
        let (peer_tx, peer_rx) = mpsc::unbounded_channel();
        (
            WireguardGatewayData {
                config,
                keypair,
                client_registry: Arc::new(DashMap::default()),
                peer_tx,
            },
            peer_rx,
        )
    }

    pub fn config(&self) -> Config {
        self.config
    }

    pub fn keypair(&self) -> &Arc<KeyPair> {
        &self.keypair
    }

    pub fn client_registry(&self) -> &Arc<GatewayClientRegistry> {
        &self.client_registry
    }

    pub fn add_peer(&self, client: &GatewayClient) -> Result<(), Error> {
        let mut peer = Peer::new(Key::new(client.pub_key.to_bytes()));
        peer.allowed_ips
            .push(IpAddrMask::new(client.private_ip, 32));
        let msg = PeerControlMessage::AddPeer(peer);
        self.peer_tx.send(msg).map_err(|_| Error::PeerModifyStopped)
    }

    pub fn remove_peer(&self, client: &GatewayClient) -> Result<(), Error> {
        let key = Key::new(client.pub_key().to_bytes());
        let msg = PeerControlMessage::RemovePeer(key);
        self.peer_tx.send(msg).map_err(|_| Error::PeerModifyStopped)
    }
}

pub struct WireguardData {
    pub inner: WireguardGatewayData,
    pub peer_rx: UnboundedReceiver<PeerControlMessage>,
}

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard(
    task_client: nym_task::TaskClient,
    wireguard_data: WireguardData,
) -> Result<std::sync::Arc<WgApiWrapper>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use defguard_wireguard_rs::{InterfaceConfiguration, WireguardInterfaceApi};
    use ip_network::IpNetwork;
    use peer_controller::PeerController;

    let mut peers = vec![];
    for peer_client in wireguard_data.inner.client_registry().iter() {
        let mut peer = Peer::new(Key::new(peer_client.pub_key.to_bytes()));
        let peer_ip_mask = IpAddrMask::new(peer_client.private_ip, 32);
        peer.set_allowed_ips(vec![peer_ip_mask]);
        peers.push(peer);
    }

    let ifname = String::from(WG_TUN_NAME);
    let wg_api = defguard_wireguard_rs::WGApi::new(ifname.clone(), false)?;
    wg_api.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.inner.keypair().private_key().to_bytes()),
        address: wireguard_data.inner.config().private_ip.to_string(),
        port: wireguard_data.inner.config().announced_port as u32,
        peers,
    };
    wg_api.configure_interface(&interface_config)?;

    // Use a dummy peer to create routing rule for the entire network space
    let mut catch_all_peer = Peer::new(Key::new([0; 32]));
    let network = IpNetwork::new_truncate(
        wireguard_data.inner.config().private_ip,
        wireguard_data.inner.config().private_network_prefix,
    )?;
    catch_all_peer.set_allowed_ips(vec![IpAddrMask::new(
        network.network_address(),
        network.netmask(),
    )]);
    wg_api.configure_peer_routing(&[catch_all_peer])?;

    let wg_api = std::sync::Arc::new(WgApiWrapper::new(wg_api));
    let mut controller = PeerController::new(wg_api.clone(), wireguard_data.peer_rx);
    tokio::spawn(async move { controller.run(task_client).await });

    Ok(wg_api)
}

#[cfg(not(target_os = "linux"))]
pub async fn start_wireguard() {
    todo!("WireGuard is currently only supported on Linux");
}
