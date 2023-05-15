// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Config, Socks5};
use crate::error::Socks5ClientCoreError;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::SphinxSocksServer,
};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_client_core::config::DebugConfig;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::{TaskClient, TaskManager};
use nym_validator_client::nyxd::QueryNyxdClient;
use nym_validator_client::Client;
use std::error::Error;

#[cfg(target_os = "android")]
use nym_client_core::client::{
    base_client::helpers::setup_empty_reply_surb_backend, base_client::storage,
    key_manager::persistence::InMemEphemeralKeys, replies::reply_storage,
};
#[cfg(target_os = "android")]
use nym_credential_storage::ephemeral_storage::EphemeralStorage;

#[cfg(not(target_os = "android"))]
use nym_client_core::client::{
    base_client::non_wasm_helpers, base_client::storage::OnDiskPersistent,
    key_manager::persistence::OnDiskKeys, replies::reply_storage::fs_backend,
};
#[cfg(not(target_os = "android"))]
use nym_credential_storage::persistent_storage::PersistentStorage;

pub mod config;
pub mod error;
pub mod socks;

// Channels used to control the main task from outside
pub type Socks5ControlMessageSender = mpsc::UnboundedSender<Socks5ControlMessage>;
pub type Socks5ControlMessageReceiver = mpsc::UnboundedReceiver<Socks5ControlMessage>;

#[cfg(target_os = "android")]
type AndroidSocks5ClientBuilder<'a> =
    BaseClientBuilder<'a, Client<QueryNyxdClient>, storage::Ephemeral>;

#[cfg(not(target_os = "android"))]
type Socks5ClientBuilder<'a> = BaseClientBuilder<'a, Client<QueryNyxdClient>, OnDiskPersistent>;

#[derive(Debug)]
pub enum Socks5ControlMessage {
    /// Tell the main task to stop
    Stop,
}

pub struct NymClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,
}

impl NymClient {
    pub fn new(config: Config) -> Self {
        NymClient { config }
    }

    pub fn start_socks5_listener(
        socks5_config: &Socks5,
        debug_config: DebugConfig,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_status: ClientState,
        self_address: Recipient,
        shutdown: TaskClient,
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
        let mut sphinx_socks = SphinxSocksServer::new(
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
        let base_builder = self.create_base_client_builder().await?;

        #[cfg(target_os = "android")]
        let base_builder = self.create_base_client_builder().await;

        let mut started_client = base_builder.start_base().await?;
        let self_address = started_client.address;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        Self::start_socks5_listener(
            self.config.get_socks5(),
            *self.config.get_debug_settings(),
            client_input,
            client_output,
            client_state,
            self_address,
            started_client.task_manager.subscribe(),
        );

        info!("Client startup finished!");
        info!("The address of this client is: {}", self_address);

        Ok(started_client.task_manager)
    }
}

#[cfg(not(target_os = "android"))]
impl NymClient {
    fn key_store(&self) -> OnDiskKeys {
        let pathfinder = ClientKeyPathfinder::new_from_config(self.config.get_base());
        OnDiskKeys::new(pathfinder)
    }

    async fn create_bandwidth_controller(
        &self,
    ) -> BandwidthController<Client<QueryNyxdClient>, PersistentStorage> {
        let storage = nym_credential_storage::initialise_persistent_storage(
            self.config.get_base().get_database_path(),
        )
        .await;

        non_wasm_helpers::create_bandwidth_controller(self.config.get_base(), storage)
    }

    async fn create_reply_storage_backend(
        &self,
    ) -> Result<fs_backend::Backend, Socks5ClientCoreError> {
        non_wasm_helpers::setup_fs_reply_surb_backend(
            self.config.get_base().get_reply_surb_database_path(),
            &self.config.get_debug_settings().reply_surbs,
        )
        .await
        .map_err(Into::into)
    }

    async fn create_base_client_builder(
        &self,
    ) -> Result<Socks5ClientBuilder, Socks5ClientCoreError> {
        // don't create bandwidth controller if credentials are disabled
        let bandwidth_controller = if self.config.get_base().get_disabled_credentials_mode() {
            None
        } else {
            Some(self.create_bandwidth_controller().await)
        };

        let key_store = self.key_store();
        let reply_storage_backend = self.create_reply_storage_backend().await?;

        Ok(BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            key_store,
            bandwidth_controller,
            reply_storage_backend,
        ))
    }
}

#[cfg(target_os = "android")]
impl NymClient {
    fn key_store(&self) -> InMemEphemeralKeys {
        InMemEphemeralKeys::new()
    }

    fn create_bandwidth_controller(
        config: &Config,
    ) -> BandwidthController<Client<QueryNyxdClient>, EphemeralStorage> {
        let storage = nym_credential_storage::initialise_ephemeral_storage();

        create_bandwidth_controller(config.get_base(), storage)
    }

    fn create_reply_storage_backend(&self) -> reply_storage::Empty {
        setup_empty_reply_surb_backend(self.config.get_debug_settings())
    }

    #[cfg(target_os = "android")]
    fn create_base_client_builder(&self) -> Socks5ClientBuilder {
        let bandwidth_controller = if self.config.get_base().get_disabled_credentials_mode() {
            None
        } else {
            Some(self.create_bandwidth_controller().await)
        };

        let key_store = self.key_store();
        let reply_storage_backend = self.create_reply_storage_backend();

        BaseClientBuilder::new_from_base_config(
            self.config.get_base(),
            key_store,
            bandwidth_controller,
            reply_storage_backend,
        )
    }
}
