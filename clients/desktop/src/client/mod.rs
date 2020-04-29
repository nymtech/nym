// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::client::cover_traffic_stream::LoopCoverTrafficStream;
use crate::client::mix_traffic::{MixMessageReceiver, MixMessageSender, MixTrafficController};
//use crate::client::provider_poller::{PolledMessagesReceiver, PolledMessagesSender};
use crate::client::received_buffer::{
    ReceivedBufferRequestReceiver, ReceivedBufferRequestSender, ReceivedMessagesBufferController,
};
use crate::client::topology_control::{
    TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use crate::config::persistence::pathfinder::ClientPathfinder;
use crate::config::{Config, SocketType};
use crate::sockets::{tcp, websocket};
use crypto::identity::MixIdentityKeyPair;
use directory_client::presence;
use futures::channel::{mpsc, oneshot};
use log::*;
use nymsphinx::chunking::split_and_prepare_payloads;
use nymsphinx::{Destination, DestinationAddressBytes};
use pemstore::pemstore::PemStore;
//use sfw_provider_requests::auth_token::AuthToken;
use gateway_client::{GatewayClient, SphinxPacketReceiver, SphinxPacketSender};
use gateway_requests::auth_token::AuthToken;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Runtime;
use topology::NymTopology;

mod cover_traffic_stream;
mod mix_traffic;
//mod provider_poller;
mod real_traffic_stream;
pub(crate) mod received_buffer;
pub(crate) mod topology_control;

pub(crate) type InputMessageSender = mpsc::UnboundedSender<InputMessage>;
pub(crate) type InputMessageReceiver = mpsc::UnboundedReceiver<InputMessage>;

pub struct NymClient {
    config: Config,
    runtime: Runtime,
    identity_keypair: MixIdentityKeyPair,

    // to be used by "send" function or socket, etc
    input_tx: Option<InputMessageSender>,

    // to be used by "receive" function or socket, etc
    receive_tx: Option<ReceivedBufferRequestSender>,
}

#[derive(Debug)]
// TODO: make fields private
pub(crate) struct InputMessage(pub Destination, pub Vec<u8>);

impl NymClient {
    fn load_identity_keys(config_file: &Config) -> MixIdentityKeyPair {
        let identity_keypair = PemStore::new(ClientPathfinder::new_from_config(&config_file))
            .read_identity()
            .expect("Failed to read stored identity key files");
        println!(
            "Public identity key: {}\n",
            identity_keypair.public_key.to_base58_string()
        );
        identity_keypair
    }

    pub fn new(config: Config) -> Self {
        let identity_keypair = Self::load_identity_keys(&config);

        NymClient {
            runtime: Runtime::new().unwrap(),
            config,
            identity_keypair,
            input_tx: None,
            receive_tx: None,
        }
    }

    #[deprecated(
        note = "SURB_IDs are irrelevant in this system design and this method alongside everything using it, should be updated accordingly"
    )]
    pub fn as_mix_destination(&self) -> Destination {
        Destination::new(
            self.identity_keypair.public_key().derive_address(),
            // TODO: what with SURBs?
            Default::default(),
        )
    }

    pub fn as_mix_destination_address(&self) -> DestinationAddressBytes {
        self.identity_keypair.public_key.derive_address()
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream<T: 'static + NymTopology>(
        &self,
        topology_accessor: TopologyAccessor<T>,
        mix_tx: MixMessageSender,
    ) {
        info!("Starting loop cover traffic stream...");
        // we need to explicitly enter runtime due to "next_delay: time::delay_for(Default::default())"
        // set in the constructor which HAS TO be called within context of a tokio runtime
        self.runtime
            .enter(|| {
                LoopCoverTrafficStream::new(
                    mix_tx,
                    self.as_mix_destination(),
                    topology_accessor,
                    self.config.get_loop_cover_traffic_average_delay(),
                    self.config.get_average_packet_delay(),
                )
            })
            .start(self.runtime.handle());
    }

    fn start_real_traffic_stream<T: 'static + NymTopology>(
        &self,
        topology_accessor: TopologyAccessor<T>,
        mix_tx: MixMessageSender,
        input_rx: InputMessageReceiver,
    ) {
        info!("Starting real traffic stream...");
        // we need to explicitly enter runtime due to "next_delay: time::delay_for(Default::default())"
        // set in the constructor which HAS TO be called within context of a tokio runtime
        self.runtime
            .enter(|| {
                real_traffic_stream::OutQueueControl::new(
                    mix_tx,
                    input_rx,
                    self.as_mix_destination(),
                    topology_accessor,
                    self.config.get_average_packet_delay(),
                    self.config.get_message_sending_average_delay(),
                )
            })
            .start(self.runtime.handle());
    }

    // buffer controlling all messages fetched from provider
    // required so that other components would be able to use them (say the websocket)
    fn start_received_messages_buffer_controller(
        &self,
        query_receiver: ReceivedBufferRequestReceiver,
        sphinx_receiver: SphinxPacketReceiver,
    ) {
        info!("Starting 'received messages buffer controller'...");
        ReceivedMessagesBufferController::new(query_receiver, sphinx_receiver)
            .start(self.runtime.handle())
    }

    fn start_gateway_client(
        &mut self,
        sphinx_packet_sender: SphinxPacketSender,
        gateway_address: url::Url,
    ) -> GatewayClient<'static, url::Url> {
        let auth_token = self
            .config
            .get_gateway_auth_token()
            .map(|str_token| AuthToken::try_from_base58_string(str_token).ok())
            .unwrap_or(None);

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.as_mix_destination_address(),
            auth_token,
            sphinx_packet_sender,
            self.config.get_gateway_response_timeout(),
        );

        let auth_token = self.runtime.block_on(async {
            gateway_client
                .establish_connection()
                .await
                .expect("could not establish initial connection with the gateway");
            gateway_client
                .perform_initial_authentication()
                .await
                .expect("could not perform initial authentication with the gateway")
        });

        // TODO: if we didn't have an auth_token initially, save it to config or something?
        info!(
            "Performed initial authentication and our auth token is {:?}",
            auth_token.to_base58_string()
        );

        gateway_client
    }

    //    // future constantly trying to fetch any received messages from the provider
    //    // the received messages are sent to ReceivedMessagesBuffer to be available to rest of the system
    //    fn start_provider_poller<T: NymTopology>(
    //        &mut self,
    //        mut topology_accessor: TopologyAccessor<T>,
    //        poller_input_tx: PolledMessagesSender,
    //    ) {
    //        info!("Starting provider poller...");
    //        // we already have our provider written in the config
    //        let provider_id = self.config.get_provider_id();
    //
    //        // TODO: a slightly more graceful termination here
    //        if !self.runtime.block_on(topology_accessor.is_routable()) {
    //            panic!(
    //                "The current network topology seem to be insufficient to route any packets through\
    //             - check if enough nodes and a sfw-provider are online"
    //            );
    //        }
    //
    //        // TODO: a slightly more graceful termination here
    //        let provider_client_listener_address = self.runtime.block_on(
    //            topology_accessor.get_provider_socket_addr(&provider_id)
    //        ).unwrap_or_else(|| panic!("Could not find provider with id {:?}. It does not seem to be present in the current network topology.\
    //        Are you sure it is still online? Perhaps try to run `nym-client init` again to obtain a new provider", provider_id));
    //
    //        let mut provider_poller = provider_poller::ProviderPoller::new(
    //            poller_input_tx,
    //            provider_client_listener_address,
    //            self.identity_keypair.public_key().derive_address(),
    //            self.config
    //                .get_provider_auth_token()
    //                .map(|str_token| AuthToken::try_from_base58_string(str_token).ok())
    //                .unwrap_or(None),
    //            self.config.get_fetch_message_delay(),
    //            self.config.get_max_response_size(),
    //        );
    //
    //        if !provider_poller.is_registered() {
    //            info!("Trying to perform initial provider registration...");
    //            self.runtime
    //                .block_on(provider_poller.perform_initial_registration())
    //                .expect("Failed to perform initial provider registration");
    //        }
    //        provider_poller.start(self.runtime.handle());
    //    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    fn start_topology_refresher<T: 'static + NymTopology>(
        &mut self,
        topology_accessor: TopologyAccessor<T>,
    ) {
        let healthcheck_keys = MixIdentityKeyPair::new();

        let topology_refresher_config = TopologyRefresherConfig::new(
            self.config.get_directory_server(),
            self.config.get_topology_refresh_rate(),
            healthcheck_keys,
            self.config.get_topology_resolution_timeout(),
            self.config.get_healthcheck_connection_timeout(),
            self.config.get_number_of_healthcheck_test_packets() as usize,
            self.config.get_node_score_threshold(),
        );
        let mut topology_refresher =
            TopologyRefresher::new(topology_refresher_config, topology_accessor);
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        info!(
            "Obtaining initial network topology from {}",
            self.config.get_directory_server()
        );
        self.runtime.block_on(topology_refresher.refresh());
        info!("Starting topology refresher...");
        topology_refresher.start(self.runtime.handle());
    }

    // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(
        &mut self,
        mix_rx: MixMessageReceiver,
        gateway_client: GatewayClient<'static, url::Url>,
    ) {
        info!("Starting mix traffic controller...");
        MixTrafficController::new(mix_rx, gateway_client).start(self.runtime.handle());
    }

    fn start_socket_listener<T: 'static + NymTopology>(
        &self,
        topology_accessor: TopologyAccessor<T>,
        received_messages_buffer_output_tx: ReceivedBufferRequestSender,
        input_tx: InputMessageSender,
    ) {
        match self.config.get_socket_type() {
            SocketType::WebSocket => {
                websocket::listener::run(
                    self.runtime.handle(),
                    self.config.get_listening_port(),
                    input_tx,
                    received_messages_buffer_output_tx,
                    self.identity_keypair.public_key().derive_address(),
                    topology_accessor,
                );
            }
            SocketType::TCP => {
                tcp::start_tcpsocket(
                    self.runtime.handle(),
                    self.config.get_listening_port(),
                    input_tx,
                    received_messages_buffer_output_tx,
                    self.identity_keypair.public_key().derive_address(),
                    topology_accessor,
                );
            }
            SocketType::None => (),
        }
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub fn send_message(&mut self, destination: Destination, message: Vec<u8>) {
        let split_message = split_and_prepare_payloads(&message);
        debug!(
            "Splitting message into {:?} fragments!",
            split_message.len()
        );
        for message_fragment in split_message {
            let input_msg = InputMessage(destination.clone(), message_fragment);
            self.input_tx
                .as_ref()
                .expect("start method was not called before!")
                .unbounded_send(input_msg)
                .unwrap()
        }
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn check_for_messages_async(&self) -> Vec<Vec<u8>> {
        let (res_tx, res_rx) = oneshot::channel();
        self.receive_tx
            .as_ref()
            .expect("start method was not called before!")
            .unbounded_send(res_tx)
            .unwrap();

        res_rx.await.unwrap()
    }

    /// blocking version of `start` method. Will run forever (or until SIGINT is sent)
    pub fn run_forever(&mut self) {
        self.start();
        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the client will terminate now (threads are not YET nicely stopped)"
        );
    }

    pub fn start(&mut self) {
        // TODO: put into config, etc.
        let gateway_url = url::Url::parse("ws://localhost:9000").unwrap();

        info!("Starting nym client");
        // channels for inter-component communication

        // mix_tx is the transmitter for any component generating sphinx packets that are to be sent to the mixnet
        // they are used by cover traffic stream and real traffic stream
        // mix_rx is the receiver used by MixTrafficController that sends the actual traffic
        let (mix_tx, mix_rx) = mpsc::unbounded();

        // sphinx_packet_tx is the transmitter of sphinx messages received from the gateway
        // sphinx_packet_rx is the receiver for said messages - used by ReceivedMessagesBuffer
        let (sphinx_packet_tx, sphinx_packet_rx) = mpsc::unbounded();

        // received_messages_buffer_output_tx is the transmitter for *REQUESTS* for messages contained in ReceivedMessagesBuffer - used by sockets
        // the requests contain a oneshot channel to send a reply on
        // received_messages_buffer_output_rx is the received for the said requests - used by ReceivedMessagesBuffer
        let (received_messages_buffer_output_tx, received_messages_buffer_output_rx) =
            mpsc::unbounded();

        // channels responsible for controlling real messages
        let (input_tx, input_rx) = mpsc::unbounded::<InputMessage>();

        // TODO: when we switch to our graph topology, we need to remember to change 'presence::Topology' type
        let shared_topology_accessor = TopologyAccessor::<presence::Topology>::new();
        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        self.start_topology_refresher(shared_topology_accessor.clone());
        self.start_received_messages_buffer_controller(
            received_messages_buffer_output_rx,
            sphinx_packet_rx,
        );
        //        self.start_provider_poller(shared_topology_accessor.clone(), sphinx_packet_tx);

        let gateway_client = self.start_gateway_client(sphinx_packet_tx, gateway_url);

        self.start_mix_traffic_controller(mix_rx, gateway_client);
        self.start_cover_traffic_stream(shared_topology_accessor.clone(), mix_tx.clone());
        self.start_real_traffic_stream(shared_topology_accessor.clone(), mix_tx, input_rx);
        self.start_socket_listener(
            shared_topology_accessor,
            received_messages_buffer_output_tx.clone(),
            input_tx.clone(),
        );
        self.input_tx = Some(input_tx);
        self.receive_tx = Some(received_messages_buffer_output_tx);

        info!("Client startup finished!");
    }
}
