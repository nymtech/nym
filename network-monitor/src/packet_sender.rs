use std::sync::Arc;

use directory_client::presence::mixnodes::MixNodePresence;
use gateway_client::GatewayClient;
use nymsphinx::{
    addressing::{clients::Recipient, nodes::NymNodeRoutingAddress},
    SphinxPacket,
};
use topology::NymTopology;

use super::{chunker, good_topology};

pub struct PacketSender {
    directory_client: Arc<directory_client::Client>,
    gateway_client: Arc<GatewayClient>,
    good_topology: NymTopology,
    self_address: Recipient,
}

impl PacketSender {
    pub fn new(
        directory_client: Arc<directory_client::Client>,
        good_topology: NymTopology,
        self_address: Recipient,
        gateway_client: Arc<GatewayClient>,
    ) -> PacketSender {
        PacketSender {
            directory_client,
            gateway_client,
            good_topology,
            self_address,
        }
    }
    /// Run some initial checks to ensure our subsequent measurements are valid.
    /// For example, we should be able to send ourselves a Sphinx packet (and receive it
    /// via the websocket, which currently fails.
    pub async fn sanity_check(&mut self) {
        let me = self.self_address.clone();
        let messages = chunker::prepare_messages("hello".to_string(), me, &self.good_topology);
        // self.send_messages(messages).await;
    }

    pub async fn send_packets_to_all_nodes(&mut self) {
        let topology = self
            .directory_client
            .get_topology()
            .await
            .expect("couldn't retrieve topology from the directory server");
        for mixnode in topology.mix_nodes {
            self.test_one_node(mixnode.to_owned()).await;
        }
    }

    // async fn send_messages(&mut self, socket_messages: Vec<(NymNodeRoutingAddress, SphinxPacket)>) {
    //     self.gateway_client
    //         .batch_send_sphinx_packets(socket_messages)
    //         .await
    //         .unwrap();
    // }

    async fn test_one_node(&mut self, mixnode: MixNodePresence) {
        println!("Testing mixnode: {}", mixnode.pub_key);
        let me = self.self_address.clone();
        let topology_to_test = good_topology::new_with_node(mixnode.clone());
        let message = mixnode.pub_key + ":4";
        let messages = chunker::prepare_messages(message, me, &topology_to_test);
        // self.send_messages(messages).await;
    }
}
