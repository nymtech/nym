use std::{net::SocketAddr, time::Duration};

use boringtun::x25519;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use tokio::{
    sync::mpsc::{self},
    time::{error::Elapsed, timeout},
};

use crate::event::Event;

// Channels that are used to communicate with the various tunnels
#[derive(Clone)]
pub struct PeerEventSender(mpsc::Sender<Event>);
pub(crate) struct PeerEventReceiver(mpsc::Receiver<Event>);

#[derive(thiserror::Error, Debug)]
pub enum PeerEventSenderError {
    #[error("timeout")]
    Timeout {
        #[from]
        source: Elapsed,
    },

    #[error("send failed: {source}")]
    SendError {
        #[from]
        source: mpsc::error::SendError<Event>,
    },
}

impl PeerEventSender {
    pub(crate) async fn send(&self, event: Event) -> Result<(), PeerEventSenderError> {
        timeout(Duration::from_millis(1000), self.0.send(event))
            .await?
            .map_err(|err| err.into())
    }
}

impl PeerEventReceiver {
    pub(crate) async fn recv(&mut self) -> Option<Event> {
        self.0.recv().await
    }
}

pub(crate) fn peer_event_channel() -> (PeerEventSender, PeerEventReceiver) {
    let (tx, rx) = mpsc::channel(16);
    (PeerEventSender(tx), PeerEventReceiver(rx))
}

pub(crate) type PeersByKey = DashMap<x25519::PublicKey, PeerEventSender>;
pub(crate) type PeersByAddr = DashMap<SocketAddr, PeerEventSender>;

#[derive(Default)]
pub(crate) struct ActivePeers {
    active_peers: PeersByKey,
    active_peers_by_addr: PeersByAddr,
}

impl ActivePeers {
    pub(crate) fn remove(&self, public_key: &x25519::PublicKey) {
        log::info!("Removing peer: {public_key:?}");
        self.active_peers.remove(public_key);
        log::warn!("TODO: remove from peers_by_ip?");
        log::warn!("TODO: remove from peers_by_addr");
    }

    pub(crate) fn insert(
        &self,
        public_key: x25519::PublicKey,
        addr: SocketAddr,
        peer_tx: PeerEventSender,
    ) {
        self.active_peers.insert(public_key, peer_tx.clone());
        self.active_peers_by_addr.insert(addr, peer_tx);
    }

    pub(crate) fn get_by_key_mut(
        &self,
        public_key: &x25519::PublicKey,
    ) -> Option<RefMut<'_, x25519::PublicKey, PeerEventSender>> {
        self.active_peers.get_mut(public_key)
    }

    pub(crate) fn get_by_addr(
        &self,
        addr: &SocketAddr,
    ) -> Option<Ref<'_, SocketAddr, PeerEventSender>> {
        self.active_peers_by_addr.get(addr)
    }
}
