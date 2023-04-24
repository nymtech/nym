// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::ClientError;
use crate::websocket;
use futures::channel::mpsc;
use log::*;
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::{
    non_wasm_helpers, BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_client_core::client::received_buffer::{
    ReceivedBufferMessage, ReceivedBufferRequestSender, ReconstructedMessagesReceiver,
};
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use nym_task::TaskManager;
use nym_validator_client::nyxd::QueryNyxdClient;
use std::error::Error;
use tokio::sync::watch::error::SendError;

pub use nym_client_core::client::key_manager::KeyManager;
use nym_credential_storage::persistent_storage::PersistentStorage;
pub use nym_sphinx::addressing::clients::Recipient;
pub use nym_sphinx::receiver::ReconstructedMessage;
use nym_validator_client::Client;

pub mod config;

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

    pub fn new_with_keys(config: Config, key_manager: KeyManager) -> Self {
        SocketClient {
            config,
            key_manager,
        }
    }

    async fn create_bandwidth_controller(
        config: &Config,
    ) -> BandwidthController<Client<QueryNyxdClient>, PersistentStorage> {
        let details = nym_network_defaults::NymNetworkDetails::new_from_env();
        let mut client_config =
            nym_validator_client::Config::try_from_nym_network_details(&details)
                .expect("failed to construct validator client config");
        let nyxd_url = config
            .get_base()
            .get_validator_endpoints()
            .pop()
            .expect("No nyxd validator endpoint provided");
        let api_url = config
            .get_base()
            .get_nym_api_endpoints()
            .pop()
            .expect("No validator api endpoint provided");
        // overwrite env configuration with config URLs
        client_config = client_config.with_urls(nyxd_url, api_url);
        let client = nym_validator_client::Client::new_query(client_config)
            .expect("Could not construct query client");
        BandwidthController::new(
            nym_credential_storage::initialise_persistent_storage(
                config.get_base().get_database_path(),
            )
            .await,
            client,
        )
    }

    fn start_websocket_listener(
        config: &Config,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_state: ClientState,
        self_address: &Recipient,
        shutdown: nym_task::TaskClient,
    ) {
        info!("Starting websocket listener...");

        let ClientInput {
            connection_command_sender,
            input_sender,
        } = client_input;

        let ClientOutput {
            received_buffer_request_sender,
        } = client_output;

        let ClientState {
            shared_lane_queue_lengths,
            reply_controller_sender,
            ..
        } = client_state;

        let websocket_handler = websocket::HandlerBuilder::new(
            input_sender,
            connection_command_sender,
            received_buffer_request_sender,
            self_address,
            shared_lane_queue_lengths,
            reply_controller_sender,
            None,
        );

        websocket::Listener::new(config.get_listening_ip(), config.get_listening_port())
            .start(websocket_handler, shutdown);
    }

    /// blocking version of `start_socket` method. Will run forever (or until SIGINT is sent)
    pub async fn run_socket_forever(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let shutdown = self.start_socket().await?;

        let res = shutdown.catch_interrupt().await;
        log::info!("Stopping nym-client");
        res
    }

    pub async fn start_socket(self) -> Result<TaskManager, ClientError> {
        if !self.config.get_socket_type().is_websocket() {
            return Err(ClientError::InvalidSocketMode);
        }

        // don't create bandwidth controller if credentials are disabled
        let bandwidth_controller = if self.config.get_base().get_disabled_credentials_mode() {
            None
        } else {
            Some(Self::create_bandwidth_controller(&self.config).await)
        };

        let base_builder = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            bandwidth_controller,
            non_wasm_helpers::setup_fs_reply_surb_backend(
                Some(self.config.get_base().get_reply_surb_database_path()),
                self.config.get_debug_settings(),
            )
            .await?,
        );

        let self_address = base_builder.as_mix_recipient();
        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        Self::start_websocket_listener(
            &self.config,
            client_input,
            client_output,
            client_state,
            &self_address,
            started_client.task_manager.subscribe(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.task_manager)
    }

    pub async fn start_direct(
        self,
        packet_type: Option<PacketType>,
    ) -> Result<DirectClient, ClientError> {
        if self.config.get_socket_type().is_websocket() {
            return Err(ClientError::InvalidSocketMode);
        }

        // don't create bandwidth controller if credentials are disabled
        let bandwidth_controller = if self.config.get_base().get_disabled_credentials_mode() {
            None
        } else {
            Some(Self::create_bandwidth_controller(&self.config).await)
        };

        let base_client = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            bandwidth_controller,
            non_wasm_helpers::setup_fs_reply_surb_backend(
                Some(self.config.get_base().get_reply_surb_database_path()),
                self.config.get_debug_settings(),
            )
            .await?,
        );

        let address = base_client.as_mix_recipient();

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
            _received_buffer_request_sender: client_output.received_buffer_request_sender,
            reconstructed_receiver,
            address,
            shutdown_notifier: started_client.task_manager,
            packet_type,
        })
    }
}

pub struct DirectClient {
    client_input: ClientInput,
    // make sure to not drop the channel
    _received_buffer_request_sender: ReceivedBufferRequestSender,
    reconstructed_receiver: ReconstructedMessagesReceiver,
    address: Recipient,

    // we need to keep reference to this guy otherwise things will start dropping
    shutdown_notifier: TaskManager,
    packet_type: Option<PacketType>,
}

impl DirectClient {
    pub fn address(&self) -> &Recipient {
        &self.address
    }

    pub fn signal_shutdown(&self) -> Result<(), SendError<()>> {
        self.shutdown_notifier.signal_shutdown()
    }

    pub async fn wait_for_shutdown(&mut self) {
        self.shutdown_notifier.wait_for_shutdown().await
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn send_regular_message(&mut self, recipient: Recipient, message: Vec<u8>) {
        let lane = TransmissionLane::General;
        let input_msg = InputMessage::new_regular(recipient, message, lane, self.packet_type);

        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn send_anonymous_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
    ) {
        let lane = TransmissionLane::General;
        let input_msg =
            InputMessage::new_anonymous(recipient, message, reply_surbs, lane, self.packet_type);

        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");
    }

    /// EXPERIMENTAL DIRECT RUST API
    /// It's untested and there are absolutely no guarantees about it (but seems to have worked
    /// well enough in local tests)
    pub async fn send_reply(&mut self, recipient_tag: AnonymousSenderTag, message: Vec<u8>) {
        let lane = TransmissionLane::General;
        let input_msg = InputMessage::new_reply(recipient_tag, message, lane, self.packet_type);

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
