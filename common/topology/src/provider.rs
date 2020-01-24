use sphinx::route::Node as SphinxNode;
use sphinx::route::NodeAddressBytes;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Client {
    pub pub_key: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub client_listener: SocketAddr,
    pub mixnet_listener: SocketAddr,
    pub pub_key: String,
    pub registered_clients: Vec<Client>,
    pub last_seen: u64,
    pub version: String,
}

impl Node {
    pub fn get_pub_key_bytes(&self) -> [u8; 32] {
        let decoded_key_bytes = base64::decode_config(&self.pub_key, base64::URL_SAFE).unwrap();
        let mut key_bytes = [0; 32];
        key_bytes.copy_from_slice(&decoded_key_bytes[..]);
        key_bytes
    }
}

impl super::Versioned for Node {
    fn get_version(&self) -> String {
        self.version.clone()
    }
}

impl Into<SphinxNode> for Node {
    fn into(self) -> SphinxNode {
        let address_bytes = addressing::encoded_bytes_from_socket_address(self.mixnet_listener);
        let key_bytes = self.get_pub_key_bytes();
        let key = sphinx::key::new(key_bytes);

        SphinxNode::new(NodeAddressBytes::from_bytes(address_bytes), key)
    }
}
