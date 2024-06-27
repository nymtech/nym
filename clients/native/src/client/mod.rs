// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::Config;
use crate::error::ClientError;
use crate::websocket;
use log::*;
use nym_client_core::client::base_client::non_wasm_helpers::default_query_dkg_client_from_config;
use nym_client_core::client::base_client::storage::OnDiskPersistent;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_sphinx::params::PacketType;
use nym_task::TaskHandle;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::error::Error;
use std::path::PathBuf;

pub use nym_sphinx::addressing::clients::Recipient;

pub mod config;

type NativeClientBuilder<'a> = BaseClientBuilder<'a, QueryHttpRpcNyxdClient, OnDiskPersistent>;

pub struct SocketClient {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    /// Optional path to a .json file containing standalone network details.
    custom_mixnet: Option<PathBuf>,
}

impl SocketClient {
    pub fn new(config: Config, custom_mixnet: Option<PathBuf>) -> Self {
        SocketClient {
            config,
            custom_mixnet,
        }
    }

    fn start_websocket_listener(
        config: &Config,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_state: ClientState,
        self_address: &Recipient,
        shutdown: nym_task::TaskClient,
        packet_type: PacketType,
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
            Some(packet_type),
        );

        websocket::Listener::new(config.socket.host, config.socket.listening_port)
            .start(websocket_handler, shutdown);
    }

    /// blocking version of `start_socket` method. Will run forever (or until SIGINT is sent)
    pub async fn run_socket_forever(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let shutdown = self.start_socket().await?;

        let res = shutdown.wait_for_shutdown().await;
        log::info!("Stopping nym-client");
        res
    }

    async fn initialise_storage(&self) -> Result<OnDiskPersistent, ClientError> {
        Ok(OnDiskPersistent::from_paths(
            self.config.storage_paths.common_paths.clone(),
            &self.config.base.debug,
        )
        .await?)
    }

    // TODO: see if this could also be shared with socks5 client / nym-sdk maybe
    async fn create_base_client_builder(&self) -> Result<NativeClientBuilder, ClientError> {
        // don't create dkg client for the bandwidth controller if credentials are disabled
        let dkg_query_client = if self.config.base.client.disabled_credentials_mode {
            None
        } else {
            Some(default_query_dkg_client_from_config(&self.config.base))
        };

        let storage = self.initialise_storage().await?;
        let user_agent = nym_bin_common::bin_info!().into();

        let mut base_client = BaseClientBuilder::new(&self.config.base, storage, dkg_query_client)
            .with_user_agent(user_agent);

        if let Some(custom_mixnet) = &self.custom_mixnet {
            base_client = base_client.with_stored_topology(custom_mixnet)?;
        }

        Ok(base_client)
    }

    pub async fn start_socket(self) -> Result<TaskHandle, ClientError> {
        if !self.config.socket.socket_type.is_websocket() {
            return Err(ClientError::InvalidSocketMode);
        }

        let base_builder = self.create_base_client_builder().await?;
        let packet_type = self.config.base.debug.traffic.packet_type;
        let mut started_client = base_builder.start_base().await?;
        let self_address = started_client.address;
        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        Self::start_websocket_listener(
            &self.config,
            client_input,
            client_output,
            client_state,
            &self_address,
            started_client.task_handle.get_handle(),
            packet_type,
        );

        info!("Client startup finished!");
        info!("The address of this client is: {self_address}");

        Ok(started_client.task_handle)
    }
}
