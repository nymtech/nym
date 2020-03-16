use crate::filter;
use sphinx::route::Node as SphinxNode;
use sphinx::route::NodeAddressBytes;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Node {
    pub location: String,
    pub host: SocketAddr,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
    pub version: String,
}

impl Node {
    pub fn get_pub_key_bytes(&self) -> [u8; 32] {
        let mut key_bytes = [0; 32];
        bs58::decode(&self.pub_key).into(&mut key_bytes).unwrap();
        key_bytes
    }
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}

impl Into<SphinxNode> for Node {
    fn into(self) -> SphinxNode {
        let address_bytes = addressing::encoded_bytes_from_socket_address(self.host);
        let key_bytes = self.get_pub_key_bytes();
        let key = sphinx::key::new(key_bytes);

        SphinxNode::new(NodeAddressBytes::from_bytes(address_bytes), key)
    }
}
