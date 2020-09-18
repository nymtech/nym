use std::collections::HashMap;

use directory_client::{Client, DirectoryClient};
use log::error;
use tokio::runtime::Runtime;
use topology::NymTopology;

mod gateway;
mod websocket;

pub struct Monitor {
    directory_uri: String,
    good_topology: NymTopology,
    websocket_uri: String,
}

impl Monitor {
    pub fn new(directory_uri: &str, good_topology: NymTopology, websocket_uri: &str) -> Monitor {
        println!("new...");

        Monitor {
            directory_uri: directory_uri.to_string(),
            good_topology,
            websocket_uri: websocket_uri.to_string(),
        }
    }

    pub fn run(&self) {
        let mut runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            let connection = websocket::Connection::new(&self.websocket_uri).await;
            let me = connection.get_self_address().await;
            println!("Retrieved self address:  {:?}", me.to_string());

            let config = directory_client::Config::new(self.directory_uri.clone());
            let directory: Client = DirectoryClient::new(config);
            let topology = directory.get_topology().await;
            println!("Retrieved topology: {:?}", topology);

            println!("Good topology is: {:?}", self.good_topology);

            println!("Finished nym network monitor startup");
            self.sanity_check();

            self.wait_for_interrupt().await
        });
    }

    /// Run some initial checks to ensure our subsequent measurements are valid
    fn sanity_check(&self) {
        self.check_goodnode_layers();
        self.ensure_good_path_works();
    }

    /// For any of this to work, our good mixnodes need to be in layers 1, 2, and 3
    fn check_goodnode_layers(&self) {
        // let topology = NymTopology::new(vec![], HashMap::new(), vec![]);
        // self.good_mixnodes
        //     .iter()
        //     .for_each(|node| self.good_mixnodes.clone())
    }

    // fn build_sphinx_packet(&self, addresses: Vec<&str>, destination: String) -> SphinxPacket {
    //     let delays = [
    //         Delay::new_from_nanos(0),
    //         Delay::new_from_nanos(0),
    //         Delay::new_from_nanos(0),
    //     ];

    //     let route: Vec<Node> = vec![];
    //     addresses.iter().for_each(|address| {
    //         let add = NodeAddressBytes::try_from_base58_string(*address).unwrap();
    //         // let key = nymsphinx::PublicKey::route.push(Node::new(add, key));
    //     });

    //     // all of the data used to create the packet was created by us
    //     let packet =
    //         nymsphinx::SphinxPacket::new("hello".as_bytes().to_vec(), &route, destination, delays)
    //             .unwrap();
    //     packet
    // }

    /// Construct a first sphinx packet using our 3 allegedly good nodes, send it, and wait for it to come back to us.
    /// If it times out, all of our subsequent measurements are going to be invalid, so we might as stop this run.
    fn ensure_good_path_works(&self) {}

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!("Received SIGINT - the network monitor will terminate now");
    }
}

#[cfg(test)]
mod constructing_monitor {
    use super::*;

    #[test]
    fn works() {
        let network_monitor = Monitor::new(
            "https://directory.nymtech.net",
            NymTopology::new(vec![], HashMap::new(), vec![]),
            "ws://localhost:1977",
        );
        assert_eq!(
            "https://directory.nymtech.net",
            network_monitor.directory_uri
        );
        assert_eq!("ws://localhost:1977", network_monitor.websocket_uri);
    }
}

#[cfg(test)]
mod building_a_sphinx_packet {
    // use super::*;

    #[test]
    fn works() {
        // let network_monitor = Monitor::new(
        //     "https://directory.nymtech.net",
        //     NymTopology::new(vec![], HashMap::new(), vec![]),
        //     "ws://localhost:1977",
        // );
        // let address1 = "CQVy5fkf4M7EdmoLvH5MJEygqiPbfavUM3NH9eGDK1kt";
        // let address2 = "GjpuFBVzk8KiNsydAaiZG3rZKsoDtv7djCRY1QatKkS5";
        // let address3 = "EV2MTs7DBi95USRNM3hM8QBRiCoYNnXBzs67YHivv3Fh";
        // let packet = network_monitor.build_sphinx_packet(vec![address1, address2, address3]);
    }
}
