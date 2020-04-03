use crate::filter;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use sphinx::route::Node as SphinxNode;
use std::convert::TryInto;
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
        let node_address_bytes = NymNodeRoutingAddress::from(self.host).try_into().unwrap();
        let key_bytes = self.get_pub_key_bytes();
        let key = sphinx::key::new(key_bytes);

        SphinxNode::new(node_address_bytes, key)
    }
}
