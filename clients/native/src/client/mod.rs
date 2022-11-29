// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::ClientError;
use crate::websocket;
use client_connections::TransmissionLane;
use client_core::client::base_client::{BaseClientBuilder, ClientInput, ClientOutput};
use client_core::client::inbound_messages::InputMessage;
use client_core::client::key_manager::KeyManager;
use client_core::client::received_buffer::{ReceivedBufferMessage, ReconstructedMessagesReceiver};
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use futures::channel::mpsc;
use gateway_client::bandwidth::BandwidthController;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::receiver::ReconstructedMessage;
use task::{wait_for_signal, ShutdownNotifier};

pub(crate) mod config;

pub struct SocketClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,
}

impl SocketClient {
    pub fn new(config: Config) -> Self {
        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        let key_manager = KeyManager::load_keys(&pathfinder).expect("failed to load stored keys");

        SocketClient {
            config,
            key_manager,
        }
    }

    async fn create_bandwidth_controller(config: &Config) -> BandwidthController {
        #[cfg(feature = "coconut")]
        let bandwidth_controller = {
            let details = network_defaults::NymNetworkDetails::new_from_env();
            let mut client_config =
                validator_client::Config::try_from_nym_network_details(&details)
                    .expect("failed to construct validator client config");
            let nymd_url = self
                .config
                .get_base()
                .get_validator_endpoints()
                .pop()
                .expect("No nymd validator endpoint provided");
            let api_url = self
                .config
                .get_base()
                .get_validator_api_endpoints()
                .pop()
                .expect("No validator api endpoint provided");
            // overwrite env configuration with config URLs
            client_config = client_config.with_urls(nymd_url, api_url);
            let client = validator_client::Client::new_query(client_config)
                .expect("Could not construct query client");
            let coconut_api_clients =
                validator_client::CoconutApiClient::all_coconut_api_clients(&client)
                    .await
                    .expect("Could not query api clients");
            BandwidthController::new(
                credential_storage::initialise_storage(config.get_base().get_database_path()).await,
                coconut_api_clients,
            )
        };
        #[cfg(not(feature = "coconut"))]
        let bandwidth_controller = BandwidthController::new(
            credential_storage::initialise_storage(config.get_base().get_database_path()).await,
        )
        .expect("Could not create bandwidth controller");
        bandwidth_controller
    }

    fn start_websocket_listener(
        config: &Config,
        client_input: ClientInput,
        client_output: ClientOutput,
        self_address: Recipient,
    ) {
        info!("Starting websocket listener...");

        let ClientInput {
            shared_lane_queue_lengths,
            connection_command_sender,
            input_sender,
        } = client_input;

        let received_buffer_request_sender = client_output.received_buffer_request_sender;

        let websocket_handler = websocket::Handler::new(
            input_sender,
            connection_command_sender,
            received_buffer_request_sender,
            self_address,
            shared_lane_queue_lengths,
        );

        websocket::Listener::new(config.get_listening_port()).start(websocket_handler);
    }

    /// blocking version of `start_socket` method. Will run forever (or until SIGINT is sent)
    pub async fn run_socket_forever(self) -> Result<(), ClientError> {
        let shutdown = self.start_socket().await?;
        wait_for_signal().await;

        println!(
            "Received signal - the client will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
        );

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        // Some of these components have shutdown signalling implemented as part of socks5 work,
        // but since it's not fully implemented (yet) for all the components of the native client,
        // we don't try to wait and instead just stop immediately.
        //log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        //shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-client");
        Ok(())
    }

    pub async fn start_socket(self) -> Result<ShutdownNotifier, ClientError> {
        if !self.config.get_socket_type().is_websocket() {
            return Err(ClientError::InvalidSocketMode);
        }

        let base_builder = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(Self::create_bandwidth_controller(&self.config).await),
        );

        let self_address = base_builder.as_mix_recipient();
        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_websocket_listener(&self.config, client_input, client_output, self_address);

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.shutdown_notifier)
    }

    pub async fn start_direct(self) -> Result<DirectClient, ClientError> {
        if self.config.get_socket_type().is_websocket() {
            return Err(ClientError::InvalidSocketMode);
        }

        let base_client = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(Self::create_bandwidth_controller(&self.config).await),
        );

        let mut started_client = base_client.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        // register our receiver
        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        client_output
            .received_buffer_request_sender
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        Ok(DirectClient {
            client_input,
            reconstructed_receiver,
            _shutdown_notifier: started_client.shutdown_notifier,
        })
    }
}

pub struct DirectClient {
    client_input: ClientInput,
    reconstructed_receiver: ReconstructedMessagesReceiver,

    // we need to keep reference to this guy otherwise things will start dropping
    _shutdown_notifier: ShutdownNotifier,
}

impl DirectClient {
    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn send_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        with_reply_surb: bool,
    ) {
        let lane = TransmissionLane::General;
        let input_msg = InputMessage::new_fresh(recipient, message, with_reply_surb, lane);

        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn send_reply(&mut self, reply_surb: ReplySurb, message: Vec<u8>) {
        let input_msg = InputMessage::new_reply(reply_surb, message);

        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    /// Note: it waits for the first occurrence of messages being sent to ourselves. If you expect multiple
    /// messages, you might have to call this function repeatedly.
    // TODO: I guess this should really return something that `impl Stream<Item=ReconstructedMessage>`
    pub async fn wait_for_messages(&mut self) -> Vec<ReconstructedMessage> {
        use futures::StreamExt;

        self.reconstructed_receiver
            .next()
            .await
            .expect("buffer controller seems to have somehow died!")
    }
}
