// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::common::bandwidth_helpers::build_bandwidth_controller;
use crate::common::helpers;
use crate::common::nodes::TestedNodeDetails;
use crate::common::probe_tests::{
    do_ping, do_socks5_connectivity_test, lp_registration_probe, wg_probe, wg_probe_lp,
};
use crate::common::types::{Entry, LpProbeResults};
use crate::config::{CredentialArgs, CredentialMode, NetstackArgs, ProbeConfig};
use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
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
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::*;

pub use crate::common::nodes::{NymApiDirectory, query_gateway_by_ip};
pub use crate::common::types::{ProbeOutcome, ProbeResult};

mod common;
pub use common::types;
pub mod config;

pub struct Probe {
    /// Entry node
    entry_node: TestedNodeDetails,
    /// Optional exit gateway node. If not provided, entry will be used
    exit_node: Option<TestedNodeDetails>,

    config: ProbeConfig,

    network: NymNetworkDetails,

    topology: Option<NymTopology>,
}

impl Probe {
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

    /// Run a probe as an NS agent, i.e. a bonded node on a known network
    pub async fn probe_run_agent(
        mut self,
        credential_args: CredentialArgs,
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

        self.do_probe_test(mixnet_client, bandwidth_provider).await
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

        self.do_probe_test(None, bandwidth_provider).await
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

        self.do_probe_test(mixnet_client, bandwidth_provider).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn do_probe_test(
        self,
        mixnet_client: Option<nym_sdk::Result<MixnetClient>>,
        bandwith_provider: Box<dyn BandwidthTicketProvider>,
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
                        credential,
                    )
                    .await
                    .unwrap_or_default();

                    // Add wg results to probe result
                    probe_result.outcome.wg = Some(outcome);
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
                    self.config.min_gateway_mixnet_performance,
                    self.config.socks5_args,
                    self.topology,
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
}
