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
use nym_client_core::client::base_client::storage::MixnetClientStorage;
use nym_client_core::client::base_client::{
    non_wasm_helpers, BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::replies::reply_storage::ReplyStorageBackend;
use nym_client_core::config::disk_persistence::key_pathfinder::ClientKeysPathfinder;
use nym_client_core::config::DebugConfig;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::PacketType;
use nym_task::{TaskClient, TaskManager};
use std::error::Error;

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

pub struct StartedSocks5Client {
    /// Handle for managing graceful shutdown of this client. If dropped, the client will be stopped.
    pub shutdown_handle: TaskManager,

    /// Address of the started client
    pub address: Recipient,
}

pub struct NymClient<S> {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    storage: S,
}

impl<S> NymClient<S>
where
    S: MixnetClientStorage + 'static,
    S::ReplyStore: Send + Sync,
    <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
    <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync,
{
    pub fn new(config: Config, storage: S) -> Self {
        NymClient { config, storage }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_socks5_listener(
        socks5_config: &config::Socks5,
        base_debug: DebugConfig,
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

        let packet_size = base_debug
            .traffic
            .secondary_packet_size
            .unwrap_or(base_debug.traffic.primary_packet_size);

        let authenticator = Authenticator::new(auth_methods, allowed_users);
        let mut sphinx_socks = NymSocksServer::new(
            socks5_config.listening_port,
            authenticator,
            socks5_config.get_provider_mix_address(),
            self_address,
            shared_lane_queue_lengths,
            socks::client::Config::new(
                packet_size,
                socks5_config.provider_interface_version,
                socks5_config.socks5_protocol_version,
                socks5_config.send_anonymously,
                socks5_config.socks5_debug,
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
        let started = self.start().await?;

        let res = started.shutdown_handle.catch_interrupt().await;
        log::info!("Stopping nym-socks5-client");
        res
    }

    // Variant of `run_forever` that listens for remote control messages
    pub async fn run_and_listen(
        self,
        mut receiver: Socks5ControlMessageReceiver,
        sender: nym_task::StatusSender,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Start the main task
        let started = self.start().await?;
        let mut shutdown = started.shutdown_handle;

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

    pub async fn start(self) -> Result<StartedSocks5Client, Socks5ClientCoreError> {
        let (key_store, reply_storage_backend, credential_store) = self.storage.into_split();

        // don't create bandwidth controller if credentials are disabled
        let bandwidth_controller = if self.config.base.client.disabled_credentials_mode {
            None
        } else {
            Some(non_wasm_helpers::create_bandwidth_controller(
                &self.config.base,
                credential_store,
            ))
        };

        let base_builder = BaseClientBuilder::<_, S>::new_from_base_config(
            &self.config.base,
            key_store,
            bandwidth_controller,
            reply_storage_backend,
        );

        let packet_type = self.config.base.debug.traffic.packet_type;
        let mut started_client = base_builder.start_base(packet_type).await?;
        let self_address = started_client.address;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        info!("Running with {packet_type} packets",);

        Self::start_socks5_listener(
            &self.config.socks5,
            self.config.base.debug,
            client_input,
            client_output,
            client_state,
            self_address,
            started_client.task_manager.subscribe(),
            packet_type,
        );

        info!("Client startup finished!");
        info!("The address of this client is: {self_address}");

        Ok(StartedSocks5Client {
            shutdown_handle: started_client.task_manager,
            address: self_address,
        })
    }
}
