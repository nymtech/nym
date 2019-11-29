use sphinx::route::{Node as SphinxNode, Destination};

pub struct DirectoryClient {}

impl DirectoryClient {
    pub fn new() -> DirectoryClient {
        DirectoryClient {}
    }

    // Hardcoded for now. Later, this should make a network request to the directory server (if one
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

fn fake_directory_mixes() -> Vec<SphinxNode> {
    let node1 = sphinx::route::Node{
        address: [0u8; 32], //"127.0.0.1:8080".as_bytes(), // start here tomorrow :)
        pub_key: Default::default()
    };
//    vec![node1]
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
        let directory = DirectoryClient::new();
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
        let directory = DirectoryClient::new();
        let expected = Destination {
            address: [0u8;32],
            identifier: [0u8; 16],
        };

        let destination = directory.get_destination();

        assert_eq!(expected, destination);
    }
}

