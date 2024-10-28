// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
// #![warn(clippy::pedantic)]
// #![warn(clippy::expect_used)]
// #![warn(clippy::unwrap_used)]

use defguard_wireguard_rs::WGApi;
#[cfg(target_os = "linux")]
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask};
use nym_crypto::asymmetric::encryption::KeyPair;
use nym_network_defaults::constants::WG_TUN_BASE_NAME;
use nym_wireguard_types::Config;
use peer_controller::PeerControlRequest;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub(crate) mod error;
pub mod peer_controller;
pub mod peer_handle;

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
    peer_tx: Sender<PeerControlRequest>,
}

impl WireguardGatewayData {
    pub fn new(config: Config, keypair: Arc<KeyPair>) -> (Self, Receiver<PeerControlRequest>) {
        let (peer_tx, peer_rx) = mpsc::channel(1);
        (
            WireguardGatewayData {
                config,
                keypair,
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

    pub fn peer_tx(&self) -> &Sender<PeerControlRequest> {
        &self.peer_tx
    }
}

pub struct WireguardData {
    pub inner: WireguardGatewayData,
    pub peer_rx: Receiver<PeerControlRequest>,
}

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard<St: nym_gateway_storage::Storage + Clone + 'static>(
    storage: St,
    all_peers: Vec<nym_gateway_storage::models::WireguardPeer>,
    task_client: nym_task::TaskClient,
    wireguard_data: WireguardData,
) -> Result<std::sync::Arc<WgApiWrapper>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use defguard_wireguard_rs::{InterfaceConfiguration, WireguardInterfaceApi};
    use ip_network::IpNetwork;
    use peer_controller::PeerController;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    let ifname = String::from(WG_TUN_BASE_NAME);
    let wg_api = defguard_wireguard_rs::WGApi::new(ifname.clone(), false)?;
    let mut peer_bandwidth_managers = HashMap::with_capacity(all_peers.len());
    let peers = all_peers
        .into_iter()
        .map(Peer::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    for peer in peers.iter() {
        let bandwidth_manager =
            PeerController::generate_bandwidth_manager(storage.clone(), &peer.public_key)
                .await?
                .map(|bw_m| Arc::new(RwLock::new(bw_m)));
        peer_bandwidth_managers.insert(peer.public_key.clone(), bandwidth_manager);
    }
    wg_api.create_interface()?;
    let interface_config = InterfaceConfiguration {
        name: ifname.clone(),
        prvkey: BASE64_STANDARD.encode(wireguard_data.inner.keypair().private_key().to_bytes()),
        address: wireguard_data.inner.config().private_ipv4.to_string(),
        port: wireguard_data.inner.config().announced_port as u32,
        peers,
        mtu: None,
    };
    wg_api.configure_interface(&interface_config)?;

    // Use a dummy peer to create routing rule for the entire network space
    let mut catch_all_peer = Peer::new(Key::new([0; 32]));
    let network_v4 = IpNetwork::new_truncate(
        wireguard_data.inner.config().private_ipv4,
        wireguard_data.inner.config().private_network_prefix,
    )?;
    let network_v6 = IpNetwork::new_truncate(
        wireguard_data.inner.config().private_ipv6,
        wireguard_data.inner.config().private_network_prefix,
    )?;
    catch_all_peer.set_allowed_ips(vec![
        IpAddrMask::new(network_v4.network_address(), network_v4.netmask()),
        IpAddrMask::new(network_v6.network_address(), network_v6.netmask()),
    ]);
    wg_api.configure_peer_routing(&[catch_all_peer])?;

    let host = wg_api.read_interface_data()?;
    let wg_api = std::sync::Arc::new(WgApiWrapper::new(wg_api));
    let mut controller = PeerController::new(
        storage,
        wg_api.clone(),
        host,
        peer_bandwidth_managers,
        wireguard_data.inner.peer_tx.clone(),
        wireguard_data.peer_rx,
        task_client,
    );
    tokio::spawn(async move { controller.run().await });

    Ok(wg_api)
}
