use std::{net::SocketAddr, time::Duration};

use boringtun::x25519;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use tokio::sync::mpsc::{self};

use crate::tun_common::{event::Event, network_table::NetworkTable};

// Registered peers
pub type PeersByIp = NetworkTable<PeerEventSender>;

// Channels that are used to communicate with the various tunnels
#[derive(Clone)]
pub struct PeerEventSender(mpsc::Sender<Event>);
pub struct PeerEventReceiver(mpsc::Receiver<Event>);

#[derive(thiserror::Error, Debug)]
pub enum PeerEventSenderError {
    #[error("send failed: timeout: {source}")]
    SendTimeoutError {
        #[from]
        source: mpsc::error::SendTimeoutError<Event>,
    },

    #[error("send failed: {source}")]
    SendError {
        #[from]
        source: mpsc::error::SendError<Event>,
    },
}

impl PeerEventSender {
    pub async fn send(&self, event: Event) -> Result<(), PeerEventSenderError> {
        Ok(self
            .0
            .send_timeout(event, Duration::from_millis(1000))
            .await?)
    }
}

impl PeerEventReceiver {
    pub async fn recv(&mut self) -> Option<Event> {
        self.0.recv().await
    }
}

pub fn peer_event_channel() -> (PeerEventSender, PeerEventReceiver) {
    let (tx, rx) = mpsc::channel(16);
    (PeerEventSender(tx), PeerEventReceiver(rx))
}

pub(crate) type PeersByKey = DashMap<x25519::PublicKey, PeerEventSender>;
pub(crate) type PeersByAddr = DashMap<SocketAddr, PeerEventSender>;

#[derive(Default)]
pub struct ActivePeers {
    active_peers: PeersByKey,
    active_peers_by_addr: PeersByAddr,
}

impl ActivePeers {
    pub fn remove(&self, public_key: &x25519::PublicKey) {
        log::info!("Removing peer: {public_key:?}");
        self.active_peers.remove(public_key);
        log::warn!("TODO: remove from peers_by_ip?");
        log::warn!("TODO: remove from peers_by_addr");
    }

    pub fn insert(
        &self,
        public_key: x25519::PublicKey,
        addr: SocketAddr,
        peer_tx: PeerEventSender,
    ) {
        self.active_peers.insert(public_key, peer_tx.clone());
        self.active_peers_by_addr.insert(addr, peer_tx);
    }

    pub fn get_by_key_mut(
        &self,
        public_key: &x25519::PublicKey,
    ) -> Option<RefMut<'_, x25519::PublicKey, PeerEventSender>> {
        self.active_peers.get_mut(public_key)
    }

    pub fn get_by_addr(&self, addr: &SocketAddr) -> Option<Ref<'_, SocketAddr, PeerEventSender>> {
        self.active_peers_by_addr.get(addr)
    }
}
