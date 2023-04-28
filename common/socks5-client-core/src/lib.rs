// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Config, Socks5};
use crate::error::Socks5ClientCoreError;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::NymSocksServer,
};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::key_manager::KeyManager;
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_credential_storage::storage::Storage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::PacketType;
use nym_task::{TaskClient, TaskManager};
use nym_validator_client::nyxd::QueryNyxdClient;
use nym_validator_client::Client;
use std::error::Error;

#[cfg(target_os = "android")]
use nym_client_core::client::base_client::helpers::setup_empty_reply_surb_backend;
#[cfg(not(target_os = "android"))]
use nym_client_core::client::base_client::non_wasm_helpers;
use nym_client_core::config::DebugConfig;

pub mod config;
pub mod error;
pub mod socks;

// Channels used to control the main task from outside
pub type Socks5ControlMessageSender = mpsc::UnboundedSender<Socks5ControlMessage>;
pub type Socks5ControlMessageReceiver = mpsc::UnboundedReceiver<Socks5ControlMessage>;

#[derive(Debug)]
pub enum Socks5ControlMessage {
    /// Tell the main task to stop
    Stop,
}

pub struct NymClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,
}

impl NymClient {
    pub fn new(config: Config) -> Self {
        let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
        let key_manager = KeyManager::load_keys(&pathfinder).expect("failed to load stored keys");

        NymClient {
            config,
            key_manager,
        }
    }

    pub fn new_with_keys(config: Config, key_manager: Option<KeyManager>) -> Self {
        let key_manager = key_manager.unwrap_or_else(|| {
            let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
            KeyManager::load_keys(&pathfinder).expect("failed to load stored keys")
        });

        NymClient {
            config,
            key_manager,
        }
    }

    async fn create_bandwidth_controller<St: Storage>(
        config: &Config,
        storage: St,
    ) -> BandwidthController<Client<QueryNyxdClient>, St> {
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

        BandwidthController::new(storage, client)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_socks5_listener(
        socks5_config: &Socks5,
        debug_config: DebugConfig,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_status: ClientState,
        self_address: Recipient,
        shutdown: TaskClient,
        packet_type: PacketType,
    ) {
        info!("Starting socks5 listener...");
        let auth_methods = vec![AuthenticationMethods::NoAuth as u8];
        let allowed_users: Vec<User> = Vec::new();

        let ClientInput {
            connection_command_sender,
            input_sender,
        } = client_input;

        let ClientOutput {
            received_buffer_request_sender,
        } = client_output;

        let ClientState {
            shared_lane_queue_lengths,
            ..
        } = client_status;

        let packet_size = debug_config
            .traffic
            .secondary_packet_size
            .unwrap_or(debug_config.traffic.primary_packet_size);

        let authenticator = Authenticator::new(auth_methods, allowed_users);
        let mut sphinx_socks = NymSocksServer::new(
            socks5_config.get_listening_port(),
            authenticator,
            socks5_config.get_provider_mix_address(),
            self_address,
            shared_lane_queue_lengths,
            socks::client::Config::new(
                packet_size,
                socks5_config.get_provider_interface_version(),
                socks5_config.get_socks5_protocol_version(),
                socks5_config.get_send_anonymously(),
                socks5_config.get_connection_start_surbs(),
                socks5_config.get_per_request_surbs(),
            ),
            shutdown.clone(),
            packet_type,
        );
        nym_task::spawn_with_report_error(
            async move {
                sphinx_socks
                    .serve(
                        input_sender,
                        received_buffer_request_sender,
                        connection_command_sender,
                    )
                    .await
            },
            shutdown,
        );
    }

    /// blocking version of `start` method. Will run forever (or until SIGINT is sent)
    pub async fn run_forever(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let shutdown = self.start().await?;

        let res = shutdown.catch_interrupt().await;
        log::info!("Stopping nym-socks5-client");
        res
    }

    // Variant of `run_forever` that listends for remote control messages
    pub async fn run_and_listen(
        self,
        mut receiver: Socks5ControlMessageReceiver,
        sender: nym_task::StatusSender,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Start the main task
        let mut shutdown = self.start().await?;

        // Listen to status messages from task, that we forward back to the caller
        shutdown.start_status_listener(sender).await;

        let res = tokio::select! {
            biased;
            message = receiver.next() => {
                log::debug!("Received message: {:?}", message);
                match message {
                    Some(Socks5ControlMessage::Stop) => {
                        log::info!("Received stop message");
                    }
                    None => {
                        log::info!("Channel closed, stopping");
                    }
                }
                Ok(())
            }
            Some(msg) = shutdown.wait_for_error() => {
                log::info!("Task error: {:?}", msg);
                Err(msg)
            }
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received SIGINT");
                Ok(())
            },
        };

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        res
    }

    pub async fn start(self) -> Result<TaskManager, Socks5ClientCoreError> {
        #[cfg(not(target_os = "android"))]
        let base_builder = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(
                Self::create_bandwidth_controller(
                    &self.config,
                    nym_credential_storage::initialise_persistent_storage(
                        self.config.get_base().get_database_path(),
                    )
                    .await,
                )
                .await,
            ),
            non_wasm_helpers::setup_fs_reply_surb_backend(
                Some(self.config.get_base().get_reply_surb_database_path()),
                self.config.get_debug_settings(),
            )
            .await?,
        );

        #[cfg(target_os = "android")]
        let base_builder = BaseClientBuilder::<_, Client<QueryNyxdClient>, _>::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(
                Self::create_bandwidth_controller(
                    &self.config,
                    nym_credential_storage::initialise_ephemeral_storage(),
                )
                .await,
            ),
            setup_empty_reply_surb_backend(self.config.get_debug_settings()),
        );

        let self_address = base_builder.as_mix_recipient();
        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        info!("{:?}", self.config.get_base().get_packet_type());

        Self::start_socks5_listener(
            self.config.get_socks5(),
            *self.config.get_debug_settings(),
            client_input,
            client_output,
            client_state,
            self_address,
            started_client.task_manager.subscribe(),
            self.config.get_base().get_packet_type(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.task_manager)
    }
}
