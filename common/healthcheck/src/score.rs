use log::error;
use sphinx::route::NodeAddressBytes;
use std::cmp::Ordering;
use std::fmt::Error;
use std::fmt::Formatter;
use std::net::SocketAddr;
use topology::mix;
use topology::provider;

// TODO: should 'nodetype' really be part of healthcheck::score

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum NodeType {
    Mix,
    MixProvider,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            NodeType::Mix => write!(f, "Mix"),
            NodeType::MixProvider => write!(f, "MixProvider"),
        }
    }
}

#[derive(Debug, Eq)]
pub(crate) struct NodeScore {
    typ: NodeType,
    pub_key: NodeAddressBytes,
    addresses: Vec<SocketAddr>,
    version: String,
    layer: String,
    packets_sent: u64,
    packets_received: u64,
}

impl Ord for NodeScore {
    // order by: version, layer, sent, received, pubkey; ignore addresses
    #[allow(clippy::comparison_chain)]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.typ > other.typ {
            return Ordering::Greater;
        } else if self.typ < other.typ {
            return Ordering::Less;
        }
        if self.version > other.version {
            return Ordering::Greater;
        } else if self.version < other.version {
            return Ordering::Less;
        }
        if self.layer > other.layer {
            return Ordering::Greater;
        } else if self.layer < other.layer {
            return Ordering::Less;
        }
        if self.packets_sent > other.packets_sent {
            return Ordering::Greater;
        } else if self.packets_sent < other.packets_sent {
            return Ordering::Less;
        }
        if self.packets_received > other.packets_received {
            return Ordering::Greater;
        } else if self.packets_received < other.packets_received {
            return Ordering::Less;
        }
        if self.pub_key > other.pub_key {
            return Ordering::Greater;
        } else if self.pub_key < other.pub_key {
            return Ordering::Less;
        }

        Ordering::Equal
    }
}

impl PartialOrd for NodeScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NodeScore {
    fn eq(&self, other: &Self) -> bool {
        self.typ == other.typ
            && self.pub_key == other.pub_key
            && self.addresses == other.addresses
            && self.version == other.version
            && self.layer == other.layer
            && self.packets_sent == other.packets_sent
            && self.packets_received == other.packets_received
    }
}

impl NodeScore {
    pub(crate) fn from_mixnode(node: mix::Node) -> Self {
        NodeScore {
            typ: NodeType::Mix,
            pub_key: NodeAddressBytes::from_base58_string(node.pub_key),
            addresses: vec![node.host],
            version: node.version,
            layer: format!("layer {}", node.layer),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    pub(crate) fn from_provider(node: provider::Node) -> Self {
        NodeScore {
            typ: NodeType::MixProvider,
            pub_key: NodeAddressBytes::from_base58_string(node.pub_key),
            addresses: vec![node.mixnet_listener, node.client_listener],
            version: node.version,
            layer: "provider".to_string(),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    pub(crate) fn increase_sent_packet_count(&mut self) {
        self.packets_sent += 1;
    }

    pub(crate) fn increase_received_packet_count(&mut self) {
        self.packets_received += 1;
    }

    pub(crate) fn score(&self) -> f64 {
        match self.packets_sent {
            0 => 0.0,
            _ => (self.packets_received as f64 / self.packets_sent as f64) * 100.0,
        }
    }

    pub(crate) fn pub_key(&self) -> NodeAddressBytes {
        self.pub_key.clone()
    }
}

impl std::fmt::Display for NodeScore {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let fmtd_addresses = match self.addresses.len() {
            1 => format!("{}", self.addresses[0]),
            2 => format!("{}, {}", self.addresses[0], self.addresses[1]),
            n => {
                error!(
                    "could not format score - node has {} addresses while only 1 or 2 are allowed!",
                    n
                );
                return Err(std::fmt::Error);
            }
        };
        let stringified_key = self.pub_key.to_base58_string();
        write!(
            f,
            "({})\t{}/{}\t({}%)\t|| {}\tv{} <{}> - {}",
            self.typ,
            self.packets_received,
            self.packets_sent,
            self.score(),
            self.layer,
            self.version,
            fmtd_addresses,
            stringified_key,
        )
    }
}
