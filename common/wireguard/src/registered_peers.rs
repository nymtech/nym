use std::{collections::HashMap, sync::Arc};

use ip_network::IpNetwork;
use nym_wireguard_types::PeerPublicKey;

pub(crate) type PeerIdx = u32;

#[derive(Debug)]
pub(crate) struct RegisteredPeer {
    pub(crate) public_key: PeerPublicKey,
    pub(crate) index: PeerIdx,
    pub(crate) allowed_ips: IpNetwork,
    // endpoint: SocketAddr,
}

#[derive(Debug, Default)]
pub(crate) struct RegisteredPeers {
    peers: HashMap<PeerPublicKey, Arc<tokio::sync::Mutex<RegisteredPeer>>>,
    peers_by_idx: HashMap<PeerIdx, Arc<tokio::sync::Mutex<RegisteredPeer>>>,
}

impl RegisteredPeers {
    pub(crate) fn contains_key(&self, public_key: &PeerPublicKey) -> bool {
        self.peers.contains_key(public_key)
    }

    pub(crate) fn next_idx(&self) -> PeerIdx {
        self.peers_by_idx.keys().max().unwrap_or(&0) + 1
    }

    pub(crate) async fn insert(&mut self, peer: Arc<tokio::sync::Mutex<RegisteredPeer>>) {
        let peer_idx = { peer.lock().await.index };
        let public_key = { peer.lock().await.public_key.clone() };
        self.peers.insert(public_key, Arc::clone(&peer));
        self.peers_by_idx.insert(peer_idx, peer);
    }

    #[allow(unused)]
    pub(crate) async fn remove(&mut self, public_key: &PeerPublicKey) {
        if let Some(peer) = self.peers.remove(public_key) {
            let peer_idx = peer.lock().await.index;
            if self.peers_by_idx.remove(&peer_idx).is_none() {
                log::error!("Removed registered peer but no registered index was found");
            }
        }
    }

    pub(crate) fn get_by_key(
        &self,
        public_key: &PeerPublicKey,
    ) -> Option<&Arc<tokio::sync::Mutex<RegisteredPeer>>> {
        self.peers.get(public_key)
    }

    pub(crate) fn get_by_idx(
        &self,
        peer_idx: PeerIdx,
    ) -> Option<&Arc<tokio::sync::Mutex<RegisteredPeer>>> {
        self.peers_by_idx.get(&peer_idx)
    }
}
