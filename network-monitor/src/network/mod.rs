use std::time::Duration;

use directory_client::{Client, DirectoryClient};
use gateway_client::GatewayClient;
use log::error;
use nymsphinx::{
    acknowledgements::AckKey, addressing::clients::Recipient,
    addressing::nodes::NymNodeRoutingAddress, preparer::MessagePreparer, SphinxPacket,
};
use rand::rngs::OsRng;
use tokio::runtime::Runtime;
use topology::NymTopology;

pub(crate) mod clients;
pub(crate) mod good_topology;
mod websocket;

const DEFAULT_RNG: OsRng = OsRng;

const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_AVERAGE_ACK_DELAY: Duration = Duration::from_millis(200);

pub struct Monitor {
    directory_uri: String,
    gateway_client: GatewayClient,
    good_topology: NymTopology,
    self_address: Option<Recipient>,
    websocket_uri: String,
}

impl Monitor {
    pub fn new(
        directory_uri: &str,
        good_topology: NymTopology,
        gateway_client: GatewayClient,
        websocket_uri: &str,
    ) -> Monitor {
        Monitor {
            directory_uri: directory_uri.to_string(),
            gateway_client,
            good_topology,
            self_address: None,
            websocket_uri: websocket_uri.to_string(),
        }
    }

    pub fn run(&mut self) {
        let mut runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            let connection = websocket::Connection::new(&self.websocket_uri).await;
            let me = connection.get_self_address().await;
            self.self_address = Some(me);
            println!("Retrieved self address:  {:?}", me.to_string());

            self.gateway_client
                .authenticate_and_start()
                .await
                .expect("Couldn't authenticate with gateway node.");
            println!("Authenticated to gateway");

            let config = directory_client::Config::new(self.directory_uri.clone());
            let directory: Client = DirectoryClient::new(config);
            let topology = directory.get_topology().await;

            self.sanity_check().await;
            self.wait_for_interrupt().await
        });
    }

    /// Run some initial checks to ensure our subsequent measurements are valid
    async fn sanity_check(&mut self) {
        let recipient = self.self_address.clone().unwrap();
        let messages = self.prepare_messages("hello".to_string(), recipient).await;
        self.send_messages(messages).await;
    }

    pub async fn prepare_messages(
        &self,
        message: String,
        recipient: Recipient,
    ) -> Vec<(NymNodeRoutingAddress, SphinxPacket)> {
        let message_bytes = message.into_bytes();

        let topology = &self.good_topology;

        let mut message_preparer = MessagePreparer::new(
            DEFAULT_RNG,
            recipient,
            DEFAULT_AVERAGE_PACKET_DELAY,
            DEFAULT_AVERAGE_ACK_DELAY,
        );

        let ack_key: AckKey = AckKey::new(&mut DEFAULT_RNG);

        let (split_message, _reply_keys) = message_preparer
            .prepare_and_split_message(message_bytes, false, &topology)
            .expect("failed to split the message");

        let mut socket_messages = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // don't bother with acks etc. for time being
            let prepared_fragment = message_preparer
                .prepare_chunk_for_sending(message_chunk, &topology, &ack_key, &recipient) //2 was  &self.ack_key
                .unwrap();

            socket_messages.push((
                prepared_fragment.first_hop_address,
                prepared_fragment.sphinx_packet,
            ));
        }
        socket_messages
    }

    async fn send_messages(&mut self, socket_messages: Vec<(NymNodeRoutingAddress, SphinxPacket)>) {
        self.gateway_client
            .batch_send_sphinx_packets(socket_messages)
            .await
            .unwrap();
    }

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
