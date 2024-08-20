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
use nym_gateway_storage::models::WireguardPeer;
use nym_wireguard_types::Config;
use peer_controller::PeerControlRequest;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

const WG_TUN_NAME: &str = "nymwg";

pub(crate) mod error;
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
    peer_tx: UnboundedSender<PeerControlRequest>,
}

impl WireguardGatewayData {
    pub fn new(
        config: Config,
        keypair: Arc<KeyPair>,
    ) -> (Self, UnboundedReceiver<PeerControlRequest>) {
        let (peer_tx, peer_rx) = mpsc::unbounded_channel();
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

    pub fn peer_tx(&self) -> &UnboundedSender<PeerControlRequest> {
        &self.peer_tx
    }
}

pub struct WireguardData {
    pub inner: WireguardGatewayData,
    pub peer_rx: UnboundedReceiver<PeerControlRequest>,
}

/// Start wireguard device
#[cfg(target_os = "linux")]
pub async fn start_wireguard<St: nym_gateway_storage::Storage + 'static>(
    storage: St,
    all_peers: Vec<WireguardPeer>,
    task_client: nym_task::TaskClient,
    wireguard_data: WireguardData,
    control_tx: UnboundedSender<peer_controller::PeerControlResponse>,
) -> Result<std::sync::Arc<WgApiWrapper>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use defguard_wireguard_rs::{InterfaceConfiguration, WireguardInterfaceApi};
    use ip_network::IpNetwork;
    use peer_controller::PeerController;

    let mut peers = vec![];
    let mut suspended_peers = vec![];
    for storage_peer in all_peers {
        let suspended = storage_peer.suspended;
        let peer = Peer::try_from(storage_peer)?;
        if suspended {
            suspended_peers.push(peer);
        } else {
            peers.push(peer);
        }
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
    let mut controller = PeerController::new(
        storage,
        wg_api.clone(),
        interface_config.peers,
        suspended_peers,
        wireguard_data.peer_rx,
        control_tx,
    );
    tokio::spawn(async move { controller.run(task_client).await });

    Ok(wg_api)
}
