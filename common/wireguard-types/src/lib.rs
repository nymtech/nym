// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use defguard_wireguard_rs::{host::Peer, key::Key, net::IpAddrMask, WGApi};
use nym_crypto::asymmetric::encryption::KeyPair;
use peer_controller::PeerControlMessage;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub mod config;
pub mod error;
pub mod peer_controller;
pub mod public_key;
pub mod registration;

pub use config::Config;
pub use error::Error;
pub use public_key::PeerPublicKey;
pub use registration::{
    ClientMac, ClientMessage, ClientRegistrationResponse, GatewayClient, GatewayClientRegistry,
    InitMessage, Nonce,
};

#[cfg(feature = "verify")]
pub use registration::HmacSha256;

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
        self.peer_tx
            .send(msg)
            .map_err(|source| Error::PeerModifyStopped { source })
    }

    pub fn remove_peer(&self, client: &GatewayClient) -> Result<(), Error> {
        let key = Key::new(client.pub_key().to_bytes());
        let msg = PeerControlMessage::RemovePeer(key);
        self.peer_tx
            .send(msg)
            .map_err(|source| Error::PeerModifyStopped { source })
    }
}

pub struct WireguardData {
    pub inner: WireguardGatewayData,
    pub peer_rx: UnboundedReceiver<PeerControlMessage>,
}
