use log::error;
use sphinx::route::NodeAddressBytes;
use std::fmt::Formatter;
use std::net::SocketAddr;
use topology::{MixNode, MixProviderNode};

#[derive(Debug)]
pub(crate) struct NodeScore {
    pub_key: NodeAddressBytes,
    addresses: Vec<SocketAddr>,
    version: String,
    layer: String,
    packets_sent: u64,
    packets_received: u64,
}

impl NodeScore {
    pub(crate) fn from_mixnode(node: MixNode) -> Self {
        NodeScore {
            pub_key: NodeAddressBytes::from_b64_string(node.pub_key),
            addresses: vec![node.host],
            version: node.version,
            layer: format!("layer {}", node.layer),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    pub(crate) fn from_provider(node: MixProviderNode) -> Self {
        NodeScore {
            pub_key: NodeAddressBytes::from_b64_string(node.pub_key),
            addresses: vec![node.mixnet_listener, node.client_listener],
            version: node.version,
            layer: format!("provider"),
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
        let health_percentage = match self.packets_sent {
            0 => 0.0,
            _ => (self.packets_received as f64 / self.packets_sent as f64) * 100.0,
        };
        let stringified_key = self.pub_key.to_b64_string();
        write!(
            f,
            "{}/{}\t({}%)\t|| {}\tv{} <{}> - {}",
            self.packets_received,
            self.packets_sent,
            health_percentage,
            self.layer,
            self.version,
            fmtd_addresses,
            stringified_key,
        )
    }
}
