// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::common::bandwidth_helpers::build_bandwidth_controller;
use crate::common::helpers;
use crate::common::nodes::TestedNodeDetails;
use crate::common::probe_tests::{
    do_ping, do_socks5_connectivity_test, lp_registration_probe, wg_probe,
};
use crate::common::types::{Entry, LpProbeResults};
use crate::config::{CredentialArgs, CredentialMode, EXIT_POLICY_PORTS, NetstackArgs, ProbeConfig};
use nym_authenticator_client::{
    AuthClientMixnetListener, AuthClientMixnetListenerHandle, AuthenticatorClient,
};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_client_core::config::ForgetMe;
use nym_config::defaults::NymNetworkDetails;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::x25519;
use nym_sdk::mixnet::{
    Ephemeral, KeyStore, MixnetClient, MixnetClientBuilder, MixnetClientStorage, StoragePaths,
};
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::*;

pub use crate::common::nodes::{NymApiDirectory, query_gateway_by_ip};
pub use crate::common::types::{PortCheckResult, PortsCheckSummary, ProbeOutcome, ProbeResult};

mod common;
pub use common::types;
pub mod config;

#[derive(Debug, Clone, Copy)]
pub enum AgentPortsSchedule {
    NsAgent { last_ports_check_utc: Option<i64> },
}

fn exit_policy_ports_check_due(last_ports_check_utc: Option<i64>, now: i64) -> bool {
    const FOURTEEN_DAYS_SECS: i64 = 14 * 24 * 60 * 60;
    match last_ports_check_utc {
        None => true,
        Some(ts) => now.saturating_sub(ts) >= FOURTEEN_DAYS_SECS,
    }
}

fn unix_timestamp_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub struct Probe {
    /// Entry node
    entry_node: TestedNodeDetails,
    /// Optional exit gateway node. If not provided, entry will be used
    exit_node: Option<TestedNodeDetails>,

    config: ProbeConfig,

    network: NymNetworkDetails,

    topology: Option<NymTopology>,
}

#[derive(Debug, Clone)]
pub struct RunPortsConfig {
    pub min_gateway_mixnet_performance: Option<u8>,
    pub ignore_egress_epoch_role: bool,
    pub netstack_args: NetstackArgs,
}

// Port checks always target a bonded gateway. There are two entry points:
//   - `run_ports`   : local CLI, on-disk storage + mnemonic.
//   - `run_ports_for_agent`: NS agent, ephemeral storage + ticket materials.

struct PortScanRun {
    can_register: bool,
    port_results: BTreeMap<String, bool>,
    last_error: Option<String>,
}

/// Validated info needed to run a WG port-check via the mixnet.
struct PortCheckSetup {
    exit_node: TestedNodeDetails,
    exit_identity: String,
    authenticator: nym_sdk::mixnet::Recipient,
    ip_address: IpAddr,
    port_check_target: String,
    ports_count: usize,
}

impl PortCheckSetup {
    fn new(exit_node: TestedNodeDetails, config: &RunPortsConfig) -> anyhow::Result<Self> {
        let exit_identity = exit_node.identity.to_string();

        let (authenticator, ip_address) =
            match (exit_node.authenticator_address, exit_node.ip_address) {
                (Some(auth), Some(ip)) => (auth, ip),
                _ => anyhow::bail!(
                    "Gateway {} missing authenticator address or IP — not a functional exit",
                    exit_identity
                ),
            };

        let ports_count = config.netstack_args.port_check_ports.len();
        if ports_count == 0 {
            anyhow::bail!(
                "No ports specified. Use --check-ports 80,443,22021 or --check-all-ports"
            );
        }

        Ok(Self {
            exit_node,
            exit_identity,
            authenticator,
            ip_address,
            port_check_target: config.netstack_args.port_check_target.clone(),
            ports_count,
        })
    }

    fn failed_to_connect(&self, err: impl std::fmt::Display) -> PortCheckResult {
        PortCheckResult {
            gateway: self.exit_identity.clone(),
            can_register: false,
            port_check_target: self.port_check_target.clone(),
            ports: BTreeMap::new(),
            error: Some(format!("Failed to connect to mixnet: {err}")),
        }
    }
}

impl Probe {
    async fn run_port_scan_with_retries(
        mixnet_listener_task: &AuthClientMixnetListenerHandle,
        nym_address: nym_sdk::mixnet::Recipient,
        authenticator: nym_sdk::mixnet::Recipient,
        authenticator_version: nym_authenticator_requests::AuthenticatorVersion,
        ip_address: IpAddr,
        bandwidth_provider: &dyn BandwidthTicketProvider,
        wg_ticket_type: TicketType,
        credential_provider: nym_sdk::mixnet::NodeIdentity,
        netstack_args: NetstackArgs,
        awg_args: Option<String>,
    ) -> PortScanRun {
        let mut port_results: BTreeMap<String, bool> = BTreeMap::new();
        let mut can_register = false;
        let mut last_error = None;
        let max_attempts = 3;

        for attempt in 1..=max_attempts {
            if attempt > 1 {
                info!("Retrying authenticator registration (attempt {attempt}/{max_attempts})...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }

            let credential = match bandwidth_provider
                .get_ecash_ticket(wg_ticket_type, credential_provider, 1)
                .await
            {
                Ok(ticket) => ticket.data,
                Err(e) => {
                    error!("Failed to get ecash ticket: {e}");
                    last_error = Some(format!("Failed to get ecash ticket: {e}"));
                    break;
                }
            };

            let mut rng = rand::thread_rng();
            let auth_client = AuthenticatorClient::new(
                mixnet_listener_task.subscribe(),
                mixnet_listener_task.mixnet_sender(),
                nym_address,
                authenticator,
                authenticator_version,
                Arc::new(x25519::KeyPair::new(&mut rng)),
                ip_address,
            );

            match wg_probe(
                auth_client,
                ip_address,
                authenticator_version,
                awg_args.clone(),
                netstack_args.clone(),
                true, // port_check_only
                credential,
            )
            .await
            {
                Ok(outcome) => {
                    if outcome.can_register {
                        can_register = true;
                        port_results = outcome
                            .port_check_results
                            .unwrap_or_default()
                            .into_iter()
                            .collect();
                        let open = port_results.values().filter(|&&v| v).count();
                        info!(
                            "Port check complete: {}/{} ports open",
                            open,
                            port_results.len()
                        );
                        break;
                    }
                    warn!(
                        "Auth registration returned but can_register=false (attempt {attempt}/{max_attempts})"
                    );
                    last_error = Some("Auth registration did not complete".into());
                }
                Err(e) => {
                    warn!("WG probe error: {e} (attempt {attempt}/{max_attempts})");
                    last_error = Some(format!("WG probe error: {e}"));
                }
            }
        }

        PortScanRun {
            can_register,
            port_results,
            last_error,
        }
    }

    /// Warm up routes, register with the authenticator, run the port scan and tear down.
    async fn port_check_after_connect(
        mixnet_client: MixnetClient,
        setup: PortCheckSetup,
        bandwidth_provider: &dyn BandwidthTicketProvider,
        netstack_args: NetstackArgs,
    ) -> PortCheckResult {
        info!("Warming up mixnet routes...");
        let nym_address = *mixnet_client.nym_address();
        let (warmup_result, mixnet_client) = do_ping(
            mixnet_client,
            nym_address,
            setup.exit_node.exit_router_address,
            false,
        )
        .await;

        match warmup_result {
            Ok(_) => info!("Mixnet warmup done"),
            Err(e) => warn!("Warmup had issues ({e}), auth may be less reliable"),
        }

        let nym_address = *mixnet_client.nym_address();
        let mixnet_listener_task =
            AuthClientMixnetListener::new(mixnet_client, CancellationToken::new()).start();

        let scan = Self::run_port_scan_with_retries(
            &mixnet_listener_task,
            nym_address,
            setup.authenticator,
            setup.exit_node.authenticator_version,
            setup.ip_address,
            bandwidth_provider,
            TicketType::V1WireguardExit,
            setup.exit_node.identity,
            netstack_args,
            None,
        )
        .await;

        mixnet_listener_task.stop().await;

        PortCheckResult {
            gateway: setup.exit_identity,
            can_register: scan.can_register,
            port_check_target: setup.port_check_target,
            ports: scan.port_results,
            error: if scan.can_register {
                None
            } else {
                scan.last_error
            },
        }
    }

    /// Create a probe with pre-queried gateway nodes
    pub fn new(
        entry_node: TestedNodeDetails,
        exit_node: Option<TestedNodeDetails>,
        network: NymNetworkDetails,
        config: ProbeConfig,
    ) -> Self {
        Self {
            entry_node,
            exit_node,
            network,
            config,
            topology: None,
        }
    }

    pub async fn new_for_agent(
        entry_gateway: nym_sdk::mixnet::ed25519::PublicKey,
        network: NymNetworkDetails,
        mut config: ProbeConfig,
    ) -> anyhow::Result<Self> {
        let api_url = network
            .endpoints
            .first()
            .and_then(|ep| ep.api_url())
            .ok_or(anyhow::anyhow!("missing api url"))?;

        let directory = NymApiDirectory::new(api_url).await?;
        let entry_details = directory
            .entry_gateway(&entry_gateway)?
            .to_testable_node()?;

        // Agents run everything
        config.test_mode = config::TestMode::All;

        Ok(Self {
            entry_node: entry_details,
            exit_node: None,
            network,
            config,
            topology: None,
        })
    }

    pub async fn probe_run_agent(
        mut self,
        credential_args: CredentialArgs,
        ports_schedule: Option<AgentPortsSchedule>,
    ) -> anyhow::Result<ProbeResult> {
        let storage = Ephemeral::default();

        let mixnet_debug_config = helpers::mixnet_debug_config(
            self.config.min_gateway_mixnet_performance,
            self.config.ignore_egress_epoch_role,
        );

        // If we need to run at least one mixnet client, prefetch topology
        if self.config.test_mode.needs_mixnet() || self.config.test_mode.socks5_tests() {
            self.topology = helpers::fetch_topology(&self.network, &mixnet_debug_config)
            .await
            .inspect_err(|e| warn!("Failed to fetch topology for that run, mixnet clients will have to handle themselves : {e}")).ok();
        }

        // Connect to the mixnet via the entry gateway
        let mut mixnet_client_builder = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(self.entry_node.identity.to_string())
            .network_details(self.network.clone())
            .debug_config(mixnet_debug_config)
            .with_forget_me(ForgetMe::new_all())
            .credentials_mode(true);

        if let Some(topology) = &self.topology {
            mixnet_client_builder = mixnet_client_builder.custom_topology_provider(Box::new(
                HardcodedTopologyProvider::new(topology.clone()),
            ));
        }

        let disconnected_mixnet_client = mixnet_client_builder.build()?;

        // Import credential
        credential_args
            .import_credential(&disconnected_mixnet_client)
            .await?;

        let bandwidth_provider =
            build_bandwidth_controller(&self.network, storage.credential_store().clone(), false)?;

        // Mixnet client start
        let mixnet_client = if self.config.test_mode.needs_mixnet() {
            Some(disconnected_mixnet_client.connect_to_mixnet().await)
        } else {
            // Make sure keys are generated, in case we don't start the mixnet client
            let key_store = storage.key_store();
            let mut rng = OsRng;
            if key_store.load_keys().await.is_err() {
                tracing::log::debug!("Generating new client keys");
                nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
            }
            None
        };

        self.do_probe_test(mixnet_client, bandwidth_provider, ports_schedule)
            .await
    }

    /// Run a probe on unannounced gateway(s) some tests will not be available
    pub async fn probe_run_locally(
        self,
        config_dir: &PathBuf,
        credential: CredentialMode,
    ) -> anyhow::Result<ProbeResult> {
        let storage_paths = StoragePaths::new_from_dir(config_dir)?;
        let storage = storage_paths
            .initialise_default_persistent_storage()
            .await?;

        // We cannot run mixnet tests on unannounced gateway, but we still need one to import credential if not using mock ecash
        let disconnected_mixnet_client = MixnetClientBuilder::new_with_storage(storage.clone())
            .credentials_mode(!credential.use_mock_ecash)
            .build()?;

        // Acquire credential if needed
        credential
            .acquire(&disconnected_mixnet_client, &storage)
            .await?;

        let bandwidth_provider = build_bandwidth_controller(
            &self.network,
            storage.credential_store().clone(),
            credential.use_mock_ecash,
        )?;

        // Make sure keys are generated
        let key_store = storage.key_store();
        let mut rng = OsRng;
        if key_store.load_keys().await.is_err() {
            tracing::log::debug!("Generating new client keys");
            nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
        }

        self.do_probe_test(None, bandwidth_provider, None).await
    }

    pub async fn probe_run(
        mut self,
        config_dir: &PathBuf,
        credential: CredentialMode,
    ) -> anyhow::Result<ProbeResult> {
        let storage_paths = StoragePaths::new_from_dir(config_dir)?;
        let storage = storage_paths
            .initialise_default_persistent_storage()
            .await?;

        let mixnet_debug_config = helpers::mixnet_debug_config(
            self.config.min_gateway_mixnet_performance,
            self.config.ignore_egress_epoch_role,
        );

        // If we need to run at least one mixnet client, prefetch topology
        if self.config.test_mode.needs_mixnet() || self.config.test_mode.socks5_tests() {
            self.topology = helpers::fetch_topology(&self.network, &mixnet_debug_config)
            .await
            .inspect_err(|e| warn!("Failed to fetch topology for that run, mixnet clients will have to handle themselves : {e}")).ok();
        }

        // Connect to the mixnet via the entry gateway, with forget-me flag only for stats so that gateway remembers client
        // and keeps its bandwidth between probe runs
        let mut mixnet_client_builder = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(self.entry_node.identity.to_string())
            .network_details(self.network.clone())
            .debug_config(mixnet_debug_config)
            .with_forget_me(ForgetMe::new_stats())
            .credentials_mode(!credential.use_mock_ecash);

        if let Some(topology) = &self.topology {
            mixnet_client_builder = mixnet_client_builder.custom_topology_provider(Box::new(
                HardcodedTopologyProvider::new(topology.clone()),
            ));
        }

        let disconnected_mixnet_client = mixnet_client_builder.build()?;
        disconnected_mixnet_client.setup_client_keys().await?;

        // Acquire credential if needed
        credential
            .acquire(&disconnected_mixnet_client, &storage)
            .await?;

        let bandwidth_provider = build_bandwidth_controller(
            &self.network,
            storage.credential_store().clone(),
            credential.use_mock_ecash,
        )?;

        // Mixnet client start
        let mixnet_client = if self.config.test_mode.needs_mixnet() {
            Some(disconnected_mixnet_client.connect_to_mixnet().await)
        } else {
            // Make sure keys are generated, in case we don't start the mixnet client
            let key_store = storage.key_store();
            let mut rng = OsRng;
            if key_store.load_keys().await.is_err() {
                tracing::log::debug!("Generating new client keys");
                nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
            }
            None
        };

        self.do_probe_test(mixnet_client, bandwidth_provider, None)
            .await
    }

    pub async fn run_ports(
        entry_node: TestedNodeDetails,
        exit_node: Option<TestedNodeDetails>,
        network: NymNetworkDetails,
        config: &RunPortsConfig,
        config_dir: &PathBuf,
        credential: CredentialMode,
    ) -> anyhow::Result<PortCheckResult> {
        let exit_node = exit_node.unwrap_or(entry_node.clone());
        let setup = PortCheckSetup::new(exit_node, config)?;

        info!(
            "Port check: testing {} ports on gateway {} via {}",
            setup.ports_count, setup.exit_identity, setup.port_check_target
        );

        let storage_paths = StoragePaths::new_from_dir(config_dir)?;
        let storage = storage_paths
            .initialise_default_persistent_storage()
            .await?;

        let mixnet_debug_config = helpers::mixnet_debug_config(
            config.min_gateway_mixnet_performance,
            config.ignore_egress_epoch_role,
        );

        let topology = helpers::fetch_topology(&network, &mixnet_debug_config)
            .await
            .inspect_err(|e| warn!("Failed to fetch topology: {e}"))
            .ok();

        let mut mixnet_client_builder = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(entry_node.identity.to_string())
            .network_details(network.clone())
            .debug_config(mixnet_debug_config)
            .with_forget_me(ForgetMe::new_stats())
            .credentials_mode(!credential.use_mock_ecash);

        if let Some(topology) = &topology {
            mixnet_client_builder = mixnet_client_builder.custom_topology_provider(Box::new(
                HardcodedTopologyProvider::new(topology.clone()),
            ));
        }

        let disconnected_mixnet_client = mixnet_client_builder.build()?;

        // make sure identity keys exist before credential acquisition
        // (acquire_bandwidth → create_bandwidth_client needs them on disk)
        let key_store = storage.key_store();
        if key_store.load_keys().await.is_err() {
            debug!("Generating new client keys");
            let mut rng = OsRng;
            nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
        }

        credential
            .acquire(&disconnected_mixnet_client, &storage)
            .await?;

        let bandwidth_provider = build_bandwidth_controller(
            &network,
            storage.credential_store().clone(),
            credential.use_mock_ecash,
        )?;

        let mixnet_client = match disconnected_mixnet_client.connect_to_mixnet().await {
            Ok(client) => {
                info!(
                    "Connected to mixnet via entry gateway: {}",
                    entry_node.identity
                );
                info!("Our nym address: {}", *client.nym_address());
                client
            }
            Err(e) => return Ok(setup.failed_to_connect(e)),
        };

        Ok(Self::port_check_after_connect(
            mixnet_client,
            setup,
            bandwidth_provider.as_ref(),
            config.netstack_args.clone(),
        )
        .await)
    }

    /// Bonded gateway port-check, run by the NS agent. Uses ephemeral storage and ticket
    /// materials provided by the NS API instead of mnemonic-based acquisition.
    pub async fn run_ports_for_agent(
        entry_gateway: nym_sdk::mixnet::ed25519::PublicKey,
        network: NymNetworkDetails,
        config: &RunPortsConfig,
        credential_args: CredentialArgs,
    ) -> anyhow::Result<PortCheckResult> {
        let api_url = network
            .endpoints
            .first()
            .and_then(|ep| ep.api_url())
            .ok_or(anyhow::anyhow!("missing api url"))?;

        let directory = NymApiDirectory::new(api_url).await?;
        let entry_node = directory
            .entry_gateway(&entry_gateway)?
            .to_testable_node()?;

        // agent always uses the entry gateway as the exit
        let setup = PortCheckSetup::new(entry_node.clone(), config)?;

        info!(
            "Port check (agent): testing {} ports on gateway {} via {}",
            setup.ports_count, setup.exit_identity, setup.port_check_target
        );

        let storage = Ephemeral::default();

        let mixnet_debug_config = helpers::mixnet_debug_config(
            config.min_gateway_mixnet_performance,
            config.ignore_egress_epoch_role,
        );

        let topology = helpers::fetch_topology(&network, &mixnet_debug_config)
            .await
            .inspect_err(|e| warn!("Failed to fetch topology: {e}"))
            .ok();

        let mut mixnet_client_builder = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(entry_node.identity.to_string())
            .network_details(network.clone())
            .debug_config(mixnet_debug_config)
            .with_forget_me(ForgetMe::new_stats())
            .credentials_mode(true);

        if let Some(topology) = &topology {
            mixnet_client_builder = mixnet_client_builder.custom_topology_provider(Box::new(
                HardcodedTopologyProvider::new(topology.clone()),
            ));
        }

        let disconnected_mixnet_client = mixnet_client_builder.build()?;

        credential_args
            .import_credential(&disconnected_mixnet_client)
            .await?;

        let bandwidth_provider =
            build_bandwidth_controller(&network, storage.credential_store().clone(), false)?;

        let mixnet_client = match disconnected_mixnet_client.connect_to_mixnet().await {
            Ok(client) => {
                info!(
                    "Connected to mixnet via entry gateway: {}",
                    entry_node.identity
                );
                info!("Our nym address: {}", *client.nym_address());
                client
            }
            Err(e) => return Ok(setup.failed_to_connect(e)),
        };

        Ok(Self::port_check_after_connect(
            mixnet_client,
            setup,
            bandwidth_provider.as_ref(),
            config.netstack_args.clone(),
        )
        .await)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn do_probe_test(
        self,
        mixnet_client: Option<nym_sdk::Result<MixnetClient>>,
        bandwith_provider: Box<dyn BandwidthTicketProvider>,
        ports_schedule: Option<AgentPortsSchedule>,
    ) -> anyhow::Result<ProbeResult> {
        // Setup exit node
        let entry_under_test = self.exit_node.is_none();
        let exit_node = self.exit_node.unwrap_or(self.entry_node.clone());

        let mut probe_result = ProbeResult {
            node: self.entry_node.identity.to_string(),
            used_entry: exit_node.identity.to_string(),
            outcome: ProbeOutcome {
                as_entry: Entry::NotTested,
                as_exit: None,
                wg: None,
                lp: None,
                socks5: None,
            },
            ports_check: None,
        };

        let mixnet_client = match mixnet_client {
            Some(Ok(mixnet_client)) => {
                // We can connect, we don't know about routing yet, but having `false` if we don't test it is weird
                probe_result.outcome.as_entry = Entry::success();
                info!(
                    "Successfully connected to entry gateway: {}",
                    self.entry_node.identity
                );
                info!("Our nym address: {}", *mixnet_client.nym_address());
                Some(mixnet_client)
            }
            Some(Err(err)) => {
                error!("Failed to connect to mixnet: {err}");
                probe_result.outcome.as_entry = if entry_under_test {
                    Entry::fail_to_connect()
                } else {
                    Entry::EntryFailure
                };
                None
            }
            None => {
                // At the moment, this is no-op. But if the initialization changes, we will have the correct value
                probe_result.outcome.as_entry = Entry::NotTested;
                None
            }
        };

        // Mixnet ping tests
        // There is some weird gymnastics with the mixnet client, but we need to give and then retrieve ownership
        let mixnet_client = if self.config.test_mode.mixnet_tests() {
            match mixnet_client {
                Some(client) => {
                    let nym_address = *client.nym_address();
                    let (outcome, client) = do_ping(
                        client,
                        nym_address,
                        exit_node.exit_router_address,
                        entry_under_test,
                    )
                    .await;
                    match outcome {
                        Ok(outcome) => {
                            probe_result.outcome = outcome;
                        }
                        Err(e) => {
                            error!("Mixnet ping tests ended with an error : {e}");
                        }
                    }
                    Some(client)
                }
                None => {
                    error!("Mixnet tests cannot be run without a mixnet client");
                    probe_result.outcome.as_entry = if entry_under_test {
                        Entry::fail_to_connect()
                    } else {
                        Entry::EntryFailure
                    };
                    None
                }
            }
        } else {
            mixnet_client
        };

        // Wireguard with Authenticator test
        if let Some(mixnet_client) = mixnet_client {
            // We have a mixnet_client to disconnect at the end here
            if self.config.test_mode.wireguard_tests() {
                if let (Some(authenticator), Some(ip_address)) =
                    (exit_node.authenticator_address, exit_node.ip_address)
                {
                    info!("Testing WireGuard via Mixnet registration");
                    // Run wireguard with authenticator
                    let nym_address = *mixnet_client.nym_address();
                    // Start the mixnet listener that the auth clients use to receive messages.
                    let mixnet_listener_task =
                        AuthClientMixnetListener::new(mixnet_client, CancellationToken::new())
                            .start();

                    let mut rng = rand::thread_rng();
                    let auth_client = AuthenticatorClient::new(
                        mixnet_listener_task.subscribe(),
                        mixnet_listener_task.mixnet_sender(),
                        nym_address,
                        authenticator,
                        exit_node.authenticator_version,
                        Arc::new(x25519::KeyPair::new(&mut rng)),
                        ip_address,
                    );

                    let (wg_ticket_type, credential_provider) = if entry_under_test {
                        (TicketType::V1WireguardEntry, self.entry_node.identity)
                    } else {
                        (TicketType::V1WireguardExit, exit_node.identity)
                    };

                    let credential = bandwith_provider
                        .get_ecash_ticket(wg_ticket_type, credential_provider, 1)
                        .await?
                        .data;

                    let outcome = wg_probe(
                        auth_client,
                        ip_address,
                        exit_node.authenticator_version,
                        self.config.amnezia_args.clone(),
                        self.config.netstack_args.clone(),
                        false,
                        credential,
                    )
                    .await
                    .unwrap_or_default();

                    // Add wg results to probe result
                    probe_result.outcome.wg = Some(outcome);

                    if let Some(AgentPortsSchedule::NsAgent {
                        last_ports_check_utc,
                    }) = ports_schedule
                    {
                        let now = unix_timestamp_secs();
                        if exit_policy_ports_check_due(last_ports_check_utc, now) {
                            if let Some(ref wg1) = probe_result.outcome.wg {
                                if wg1.can_register {
                                    info!(
                                        "Running scheduled exit-policy port scan (stale or unset last_ports_check_utc)"
                                    );
                                    let mut netstack_ports = self.config.netstack_args.clone();
                                    netstack_ports.port_check_ports = EXIT_POLICY_PORTS.to_vec();

                                    let credential2 = bandwith_provider
                                        .get_ecash_ticket(wg_ticket_type, credential_provider, 1)
                                        .await?
                                        .data;

                                    let mut rng2 = rand::thread_rng();
                                    let auth_client2 = AuthenticatorClient::new(
                                        mixnet_listener_task.subscribe(),
                                        mixnet_listener_task.mixnet_sender(),
                                        nym_address,
                                        authenticator,
                                        exit_node.authenticator_version,
                                        Arc::new(x25519::KeyPair::new(&mut rng2)),
                                        ip_address,
                                    );

                                    match wg_probe(
                                        auth_client2,
                                        ip_address,
                                        exit_node.authenticator_version,
                                        self.config.amnezia_args.clone(),
                                        netstack_ports,
                                        true,
                                        credential2,
                                    )
                                    .await
                                    {
                                        Ok(scan) => {
                                            if let Some(ref mut wg) = probe_result.outcome.wg {
                                                wg.port_check_results =
                                                    scan.port_check_results.clone();
                                            }
                                            probe_result.ports_check = Some(
                                                match &scan.port_check_results {
                                                    Some(m) if !m.is_empty() => {
                                                        PortsCheckSummary::from_port_map(
                                                            scan.can_register,
                                                            m,
                                                        )
                                                    }
                                                    _ => PortsCheckSummary::probe_error(
                                                        scan.can_register,
                                                        "exit-policy port scan returned no per-port data",
                                                    ),
                                                },
                                            );
                                        }
                                        Err(err) => {
                                            warn!("Scheduled exit-policy port scan failed: {err}");
                                            probe_result.ports_check =
                                                Some(PortsCheckSummary::probe_error(
                                                    false,
                                                    format!(
                                                        "exit-policy port scan failed: {err:#}"
                                                    ),
                                                ));
                                        }
                                    }
                                }
                            }
                        } else {
                            trace!(
                                "Skipping exit-policy port scan: checked within the last 14 days"
                            );
                        }
                    }

                    mixnet_listener_task.stop().await;
                } else {
                    warn!("Not enough information to run WireGuard via mixnet registration tests");
                    mixnet_client.disconnect().await;
                }
            } else {
                // We are not running WG tests, we don't need the mixnet client anmore
                mixnet_client.disconnect().await;
            }
        }

        // At this point, any mixnet client MUST be disconnected

        // Test LP registration if node has LP address
        if self.config.test_mode.lp_tests() {
            if let Some(lp_data) = self.entry_node.lp_data {
                info!("Node has LP data, testing LP registration...");

                let outcome =
                    lp_registration_probe(self.entry_node.identity, lp_data, &bandwith_provider)
                        .await
                        .unwrap_or_default();

                probe_result.outcome.lp = Some(outcome);
            } else {
                warn!("LP test was requested, but node did not have LP data");

                probe_result.outcome.lp = Some(LpProbeResults {
                    can_connect: false,
                    can_handshake: false,
                    can_register: false,
                    error: Some("no LP data".into()),
                })
            };
        }

        // Test socks5 connectivity
        if self.config.test_mode.socks5_tests() {
            // test failure doesn't stop further tests
            if let Some(network_requester) = exit_node.network_requester_address {
                match do_socks5_connectivity_test(
                    &network_requester,
                    self.entry_node.identity,
                    self.network.clone(),
                    self.config.socks5_args,
                )
                .await
                {
                    Ok(results) => probe_result.outcome.socks5 = Some(results),
                    Err(e) => {
                        error!("SOCKS5 test failed: {}", e);
                    }
                }
            } else {
                warn!("No NR available, skipping SOCKS5 tests");
            }
        }

        Ok(probe_result)
    }

    pub fn config(&self) -> &ProbeConfig {
        &self.config
    }
}
