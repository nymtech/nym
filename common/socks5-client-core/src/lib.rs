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
use nym_client_core::client::base_client::storage::gateway_details::GatewayDetailsStore;
use nym_client_core::client::base_client::storage::MixnetClientStorage;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::replies::reply_storage::ReplyStorageBackend;
use nym_client_core::config::DebugConfig;
use nym_client_core::init::GatewaySetup;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::params::PacketType;
use nym_task::{TaskClient, TaskManager};
use nym_topology::nym_topology_from_detailed;
use nym_topology::{
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use url::Url;

use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{Addr, Coin, Layer, MixId, MixNode};
use nym_validator_client::models::NodePerformance;

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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct PrettyDetailedMixNodeBond {
    pub mix_id: MixId,
    pub location: Option<Location>,
    pub status: MixnodeStatus,
    pub pledge_amount: Coin,
    pub total_delegation: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub mix_node: MixNode,
    pub stake_saturation: f32,
    pub uncapped_saturation: f32,
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
    pub estimated_operator_apy: f64,
    pub estimated_delegators_apy: f64,
    pub operating_cost: Coin,
    pub profit_margin_percent: Percent,
    pub family_id: Option<u16>,
    pub blacklisted: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize)]
pub(crate) struct Location {
    pub(crate) two_letter_iso_country_code: String,
    pub(crate) three_letter_iso_country_code: String,
    pub(crate) country_name: String,
    pub(crate) latitude: Option<f64>,
    pub(crate) longitude: Option<f64>,
}

struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: String,
}

impl GeoAwareTopologyProvider {
    fn new(nym_api_url: Url, filter_on: String) -> GeoAwareTopologyProvider {
        GeoAwareTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(nym_api_url),
            filter_on,
        }
    }

    async fn get_topology(&self) -> Option<NymTopology> {
        let mixnodes = match self.validator_client.get_cached_active_mixnodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_cached_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        // Also fetch mixnodes cached by explorer-api, with the purpose of getting their
        // geolocation.
        debug!("Fetching mixnodes from explorer-api...");
        let mixnodes_from_explorer_api =
            reqwest::get("https://explorer.nymtech.net/api/v1/mix-nodes")
                .await
                .unwrap()
                .json::<Vec<PrettyDetailedMixNodeBond>>()
                .await
                .unwrap();

        // Partition mixnodes_from_explorer_api according to the value of two_letter_iso_country_code
        let mixnodes_from_explorer_api_by_continent = mixnodes_from_explorer_api.into_iter().fold(
            HashMap::<String, Vec<MixId>>::new(),
            |mut acc, m| {
                if let Some(ref location) = m.location {
                    let country_code = location.two_letter_iso_country_code.clone();
                    if let Some(continent_code) = country_code_to_continent_code(&country_code) {
                        let mixnodes = acc.entry(continent_code).or_insert_with(Vec::new);
                        mixnodes.push(m.mix_id);
                    }
                }
                acc
            },
        );

        // Create a string with the number of mixnodes per continent.
        let mixnodes_from_explorer_api_by_continent_string =
            mixnodes_from_explorer_api_by_continent
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.len()))
                .collect::<Vec<_>>()
                .join(", ");
        debug!(
            "Mixnode distribution - {}",
            mixnodes_from_explorer_api_by_continent_string
        );

        // Filter mixnodes so that only the items that also exist in the mixnodes_from_explorer_api_by_continent for the key given by filter_on.
        let mixnodes = mixnodes
            .into_iter()
            .filter(|m| {
                if let Some(ids) = mixnodes_from_explorer_api_by_continent.get(&self.filter_on) {
                    ids.contains(&m.mix_id())
                } else {
                    // If the key is not setup, or no mixnodes exist for the key, then return false.
                    false
                }
            })
            .collect::<Vec<_>>();

        // Check layer distribution
        let mut layer_counts = mixnodes.iter().map(|m| m.layer()).fold(
            HashMap::<Layer, usize>::new(),
            |mut acc, layer| {
                let count = acc.entry(layer).or_insert(0);
                *count += 1;
                acc
            },
        );

        // and check the integrity
        for layer in &[Layer::One, Layer::Two, Layer::Three] {
            layer_counts.entry(*layer).or_insert(0);
            let count = layer_counts[layer];
            if count < 2 {
                error!("There are only {} mixnodes in layer {:?}", count, layer);
                return None;
            }
        }

        Some(nym_topology_from_detailed(mixnodes, gateways))
    }
}

// We map contry codes to continent codes, but we do it manually to reserve the right to tweak this
// distribution for our purposes.
// Also, at the time of writing I didn't find a simple crate that did this mapping...
fn country_code_to_continent_code(country_code: &str) -> Option<String> {
    match country_code {
        // Europe
        "AT" => Some("EU".to_string()),
        "BG" => Some("EU".to_string()),
        "CH" => Some("EU".to_string()),
        "CY" => Some("EU".to_string()),
        "CZ" => Some("EU".to_string()),
        "DE" => Some("EU".to_string()),
        "DK" => Some("EU".to_string()),
        "ES" => Some("EU".to_string()),
        "FI" => Some("EU".to_string()),
        "FR" => Some("EU".to_string()),
        "GB" => Some("EU".to_string()),
        "GR" => Some("EU".to_string()),
        "IE" => Some("EU".to_string()),
        "IT" => Some("EU".to_string()),
        "LT" => Some("EU".to_string()),
        "LU" => Some("EU".to_string()),
        "LV" => Some("EU".to_string()),
        "MD" => Some("EU".to_string()),
        "MT" => Some("EU".to_string()),
        "NL" => Some("EU".to_string()),
        "NO" => Some("EU".to_string()),
        "PL" => Some("EU".to_string()),
        "RO" => Some("EU".to_string()),
        "SE" => Some("EU".to_string()),
        "SK" => Some("EU".to_string()),
        "TR" => Some("EU".to_string()),
        "UA" => Some("EU".to_string()),

        // North America
        "CA" => Some("NA".to_string()),
        "MX" => Some("NA".to_string()),
        "US" => Some("NA".to_string()),

        // South America
        "AR" => Some("SA".to_string()),
        "BR" => Some("SA".to_string()),
        "CL" => Some("SA".to_string()),
        "CO" => Some("SA".to_string()),
        "CR" => Some("SA".to_string()),
        "GT" => Some("SA".to_string()),

        // Oceania
        "AU" => Some("OC".to_string()),

        // Asia
        "AM" => Some("AS".to_string()),
        "BH" => Some("AS".to_string()),
        "CN" => Some("AS".to_string()),
        "GE" => Some("AS".to_string()),
        "HK" => Some("AS".to_string()),
        "ID" => Some("AS".to_string()),
        "IL" => Some("AS".to_string()),
        "IN" => Some("AS".to_string()),
        "JP" => Some("AS".to_string()),
        "KH" => Some("AS".to_string()),
        "KR" => Some("AS".to_string()),
        "KZ" => Some("AS".to_string()),
        "MY" => Some("AS".to_string()),
        "RU" => Some("AS".to_string()),
        "SG" => Some("AS".to_string()),
        "TH" => Some("AS".to_string()),
        "VN" => Some("AS".to_string()),

        // Africa
        "SC" => Some("AF".to_string()),
        "UG" => Some("AF".to_string()),
        "ZA" => Some("AF".to_string()),

        _ => {
            println!("Unknown country code: {}", country_code);
            None
        }
    }
}

#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}

pub struct NymClient<S> {
    /// Client configuration options, including, among other things, packet sending rates,
    /// key filepaths, etc.
    config: Config,

    storage: S,

    setup_method: GatewaySetup,
}

impl<S> NymClient<S>
where
    S: MixnetClientStorage + 'static,
    S::ReplyStore: Send + Sync,
    <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
    <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    <S::GatewayDetailsStore as GatewayDetailsStore>::StorageError: Sync + Send,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync,
{
    pub fn new(config: Config, storage: S) -> Self {
        NymClient {
            config,
            storage,
            setup_method: GatewaySetup::MustLoad,
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
        // don't create dkg client for the bandwidth controller if credentials are disabled
        let dkg_query_client = if self.config.base.client.disabled_credentials_mode {
            None
        } else {
            Some(default_query_dkg_client_from_config(&self.config.base))
        };

        // WIP(JON)
        let nym_api = "https://validator.nymtech.net/api/".parse().unwrap();
        let filter_on = "EU";
        let topology_provider = GeoAwareTopologyProvider::new(nym_api, filter_on.to_string());

        let base_builder =
            BaseClientBuilder::new(&self.config.base, self.storage, dkg_query_client)
                .with_gateway_setup(self.setup_method)
                .with_topology_provider(Box::new(topology_provider));

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
