use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use libp2p::core::ConnectedPoint;
use libp2p::Multiaddr;
use serde::Serialize;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize)]
pub(crate) enum Endpoint {
    Dialer {
        address: Multiaddr,
    },
    Listener {
        local_address: Multiaddr,
        remote_address: Multiaddr,
    },
}

impl From<ConnectedPoint> for Endpoint {
    fn from(connected_point: ConnectedPoint) -> Self {
        match connected_point {
            ConnectedPoint::Dialer { address, .. } => Endpoint::Dialer { address },
            ConnectedPoint::Listener {
                local_addr,
                send_back_addr,
            } => Endpoint::Listener {
                local_address: local_addr,
                remote_address: send_back_addr,
            },
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct Connections {
    dialer: HashSet<Endpoint>,
    listener: HashSet<Endpoint>,
}

impl Connections {
    fn insert(&mut self, connected_point: Endpoint) {
        match connected_point {
            Endpoint::Dialer { .. } => {
                self.dialer.insert(connected_point);
            }
            Endpoint::Listener { .. } => {
                self.listener.insert(connected_point);
            }
        }
    }

    fn remove(&mut self, connected_point: &Endpoint) {
        match connected_point {
            Endpoint::Dialer { .. } => {
                self.dialer.remove(connected_point);
            }
            Endpoint::Listener { .. } => {
                self.listener.remove(connected_point);
            }
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct ConnectedPeers {
    connections: HashMap<libp2p_identity::PeerId, Connections>,
}

impl ConnectedPeers {
    pub(crate) fn is_peer_connected(&self, peer_id: &libp2p_identity::PeerId) -> bool {
        self.connections.contains_key(peer_id)
    }

    pub(crate) fn all_connected_peers_ref(&self) -> Vec<&libp2p_identity::PeerId> {
        self.connections.keys().collect()
    }

    pub(crate) fn insert(&mut self, peer_id: libp2p_identity::PeerId, connected_point: Endpoint) {
        let connections = self.connections.entry(peer_id).or_default();
        connections.insert(connected_point);
    }

    pub(crate) fn remove(&mut self, peer_id: &libp2p_identity::PeerId, connected_point: &Endpoint) {
        if let Some(connections) = self.connections.get_mut(peer_id) {
            connections.remove(connected_point);
        }
    }
}
