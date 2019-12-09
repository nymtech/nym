use sphinx::route::{Node as SphinxNode, Destination};
use crate::clients::directory::models::Topology;

use serde::Deserialize;

mod models;

pub struct Client {
//    topology: Topology,
}

impl Client {
    pub fn new() -> Client {
        let topology = retrieve_topology().unwrap();
        Client {}
    }

    // Hardcoded for now. Later, this should make a network request to the clients.directory server (if one
    // has not yet been made), parse the returned JSON, memoize the full tree of active nodes,
    // and return the list of currently active mix nodes.
    pub fn get_mixes(&self) -> Vec<SphinxNode> {
        fake_directory_mixes()
    }

    pub fn get_destination(&self) -> Destination {
        Destination {
            address: [0u8;32],
            identifier: [0u8; 16],
        }
    }
}

fn retrieve_topology() -> Result<Topology, reqwest::Error> {
    let topology: Topology = reqwest::get("https://directory.nymtech.net/api/presence/topology")?
        .json()?;
    Ok(topology)
}

fn fake_directory_mixes() -> Vec<SphinxNode> {
    let node1 = sphinx::route::Node{
        address: [0u8; 32], //"127.0.0.1:8080".as_bytes(), // start here tomorrow :)
        pub_key: Default::default()
    };
    let node2 = sphinx::route::Node{
        address: [1u8;32],
        pub_key: Default::default()
    };
    vec![node1, node2]
}

#[cfg(test)]
mod retrieving_mixnode_list {
    use super::*;

    #[test]
    fn always_works_because_it_is_hardcoded() {
        let directory = Client::new();
        let expected = fake_directory_mixes().as_slice().to_owned();

        let mixes = directory.get_mixes();

        assert_eq!(expected, mixes.as_slice());
    }
}

#[cfg(test)]
mod retrieving_destinations {
    use super::*;
    
    #[test]
    fn always_works_because_it_is_hardcoded() {
        let directory = Client::new();
        let expected = Destination {
            address: [0u8;32],
            identifier: [0u8; 16],
        };

        let destination = directory.get_destination();

        assert_eq!(expected, destination);
    }
}

