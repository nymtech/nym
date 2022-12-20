// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::Socks5ClientError;
use crate::socks;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::SphinxSocksServer,
};
use client_core::client::base_client::{
    non_wasm_helpers, BaseClientBuilder, ClientInput, ClientOutput,
};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use futures::channel::mpsc;
use futures::StreamExt;
use gateway_client::bandwidth::BandwidthController;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use std::error::Error;
use task::{wait_for_signal_and_error, ShutdownListener, ShutdownNotifier};

pub mod config;

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

    async fn create_bandwidth_controller(config: &Config) -> BandwidthController {
        #[cfg(feature = "coconut")]
        let bandwidth_controller = {
            let details = network_defaults::NymNetworkDetails::new_from_env();
            let mut client_config =
                validator_client::Config::try_from_nym_network_details(&details)
                    .expect("failed to construct validator client config");
            let nymd_url = config
                .get_base()
                .get_validator_endpoints()
                .pop()
                .expect("No nymd validator endpoint provided");
            let api_url = config
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

    fn start_socks5_listener(
        config: &Config,
        client_input: ClientInput,
        client_output: ClientOutput,
        self_address: Recipient,
        shutdown: ShutdownListener,
    ) {
        info!("Starting socks5 listener...");
        let auth_methods = vec![AuthenticationMethods::NoAuth as u8];
        let allowed_users: Vec<User> = Vec::new();

        let ClientInput {
            shared_lane_queue_lengths,
            connection_command_sender,
            input_sender,
        } = client_input;

        let received_buffer_request_sender = client_output.received_buffer_request_sender;

        let authenticator = Authenticator::new(auth_methods, allowed_users);
        let mut sphinx_socks = SphinxSocksServer::new(
            config.get_listening_port(),
            authenticator,
            config.get_provider_mix_address(),
            self_address,
            shared_lane_queue_lengths,
            socks::client::Config::new(
                config.get_send_anonymously(),
                config.get_connection_start_surbs(),
                config.get_per_request_surbs(),
            ),
            shutdown.clone(),
        );
        task::spawn_with_report_error(
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
        let mut shutdown = self.start().await?;

        let res = wait_for_signal_and_error(&mut shutdown).await;

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym-socks5-client");
        res
    }

    // Variant of `run_forever` that listends for remote control messages
    pub async fn run_and_listen(
        self,
        mut receiver: Socks5ControlMessageReceiver,
        sender: task::StatusSender,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Start the main task
        let mut shutdown = self.start().await?;

        // Listen to status messages from task, that we forward back to the caller
        shutdown.start_status_listener(sender);

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

    pub async fn start(self) -> Result<ShutdownNotifier, Socks5ClientError> {
        let base_builder = BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            self.key_manager,
            Some(Self::create_bandwidth_controller(&self.config).await),
            non_wasm_helpers::setup_fs_reply_surb_backend(
                self.config.get_base().get_reply_surb_database_path(),
                self.config.get_debug_settings(),
            )
            .await?,
        );

        let self_address = base_builder.as_mix_recipient();
        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_socks5_listener(
            &self.config,
            client_input,
            client_output,
            self_address,
            started_client.shutdown_notifier.subscribe(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.shutdown_notifier)
    }
}
