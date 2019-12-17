use crate::clients::directory;
use crate::clients::directory::presence::Topology;
use crate::clients::directory::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::clients::directory::DirectoryClient;
use crate::utils::bytes;
use curve25519_dalek::montgomery::MontgomeryPoint;
use sphinx::route::Node as SphinxNode;
use std::net::SocketAddrV4;
use std::string::ToString;

pub(crate) fn get_topology(is_local: bool) -> Topology {
    let url = if is_local {
        "http://localhost:8080".to_string()
    } else {
        "https://directory.nymtech.net".to_string()
    };
    println!("Using directory server: {:?}", url);
    let directory_config = directory::Config { base_url: url };
    let directory = directory::Client::new(directory_config);

    let topology = directory
        .presence_topology
        .get()
        .expect("Failed to retrieve network topology.");
    topology
}

pub(crate) fn route_from(topology: &Topology, route_len: usize) -> Vec<SphinxNode> {
    let mut route = vec![];
    let nodes = topology.mix_nodes.iter();
    for mix in nodes.take(route_len) {
        let address_bytes = socket_bytes_from_string(mix.host.clone());
        let decoded_key_bytes = base64::decode_config(&mix.pub_key, base64::URL_SAFE).unwrap();
        let key_bytes = bytes::zero_pad_to_32(decoded_key_bytes);
        let key = MontgomeryPoint(key_bytes);
        let sphinx_node = SphinxNode {
            address: address_bytes,
            pub_key: key,
        };
        route.push(sphinx_node);
    }
    route
}

pub(crate) fn socket_bytes_from_string(address: String) -> [u8; 32] {
    let socket: SocketAddrV4 = address.parse().unwrap();
    let host_bytes = socket.ip().octets();
    let port_bytes = socket.port().to_be_bytes();
    let mut address_bytes = [0u8; 32];

    address_bytes[0] = host_bytes[0];
    address_bytes[1] = host_bytes[1];
    address_bytes[2] = host_bytes[2];
    address_bytes[3] = host_bytes[3];

    address_bytes[4] = port_bytes[0];
    address_bytes[5] = port_bytes[1];
    address_bytes
}
