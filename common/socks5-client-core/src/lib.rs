// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::error::Socks5ClientCoreError;
use crate::socks::{
    authentication::{AuthenticationMethods, Authenticator, User},
    server::NymSocksServer,
};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_client_core::client::base_client::non_wasm_helpers::default_query_dkg_client_from_config;
use nym_client_core::client::base_client::storage::GatewaysDetailsStore;
use nym_client_core::client::base_client::storage::MixnetClientStorage;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::replies::reply_storage::ReplyStorageBackend;
use nym_client_core::config::DebugConfig;
use nym_client_core::init::types::GatewaySetup;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::PacketType;
use nym_task::manager::TaskStatus;
use nym_task::{TaskClient, TaskHandle};

use anyhow::anyhow;
use nym_validator_client::UserAgent;
use std::error::Error;
use std::path::PathBuf;

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
    pub shutdown_handle: TaskHandle,

    /// Address of the started client
    pub address: Recipient,
}

pub struct NymClient<S> {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    storage: S,

    setup_method: GatewaySetup,

    user_agent: UserAgent,

    /// Optional path to a .json file containing standalone network details.
    custom_mixnet: Option<PathBuf>,
}

impl<S> NymClient<S>
where
    S: MixnetClientStorage + 'static,
    S::ReplyStore: Send + Sync,
    <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
    <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Sync + Send,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync,
{
    pub fn new(
        config: Config,
        storage: S,
        user_agent: UserAgent,
        custom_mixnet: Option<PathBuf>,
    ) -> Self {
        NymClient {
            config,
            storage,
            setup_method: GatewaySetup::MustLoad { gateway_id: None },
            user_agent,
            custom_mixnet,
        }
    }

    pub fn with_gateway_setup(mut self, setup: GatewaySetup) -> Self {
        self.setup_method = setup;
        self
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
            socks5_config.bind_address,
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

        let res = started.shutdown_handle.wait_for_shutdown().await;
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
        let mut shutdown = started
            .shutdown_handle
            .try_into_task_manager()
            .ok_or(anyhow!(
                "attempted to use `run_and_listen` without owning shutdown handle"
            ))?;

        // Listen to status messages from task, that we forward back to the caller
        shutdown
            .start_status_listener(sender, TaskStatus::Ready)
            .await;

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
        // don't create dkg client for the bandwidth controller if credentials are disabled
        let dkg_query_client = if self.config.base.client.disabled_credentials_mode {
            None
        } else {
            Some(default_query_dkg_client_from_config(&self.config.base))
        };

        let mut base_builder =
            BaseClientBuilder::new(&self.config.base, self.storage, dkg_query_client, None)
                .with_gateway_setup(self.setup_method)
                .with_user_agent(self.user_agent);

        if let Some(custom_mixnet) = &self.custom_mixnet {
            base_builder = base_builder.with_stored_topology(custom_mixnet)?;
        }

        let packet_type = self.config.base.debug.traffic.packet_type;
        let mut started_client = base_builder.start_base().await?;
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
            started_client.task_handle.get_handle(),
            packet_type,
        );

        info!("Client startup finished!");
        info!("The address of this client is: {self_address}");

        Ok(StartedSocks5Client {
            shutdown_handle: started_client.task_handle,
            address: self_address,
        })
    }
}
