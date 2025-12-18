// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::common::helpers;
use crate::common::probe_tests::{
    do_ping, do_socks5_connectivity_test, lp_registration_probe, wg_probe, wg_probe_lp,
};
use crate::common::types::{Entry, Exit, Socks5ProbeResults, WgProbeResults};
use crate::config::Socks5Args;
use anyhow::bail;
use nym_api_requests::models::NetworkRequesterDetailsV1;
use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
use nym_client_core::config::ForgetMe;
use nym_config::defaults::NymNetworkDetails;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_sdk::mixnet::{
    CredentialStorage, Ephemeral, KeyStore, MixnetClient, MixnetClientBuilder, MixnetClientStorage,
    NodeIdentity, StoragePaths,
};
use nym_topology::NymTopology;
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::*;
use url::Url;

mod common;
pub mod config;

use crate::common::bandwidth_helpers::{acquire_bandwidth, import_bandwidth};
pub use crate::common::nodes::{
    DirectoryNode, NymApiDirectory, TestedNode, TestedNodeDetails, TestedNodeLpDetails,
    query_gateway_by_ip,
};
pub use crate::common::types::{IpPingReplies, ProbeOutcome, ProbeResult};
pub use crate::config::{CredentialArgs, NetstackArgs, TestMode};

pub struct Probe {
    entrypoint: NodeIdentity,
    tested_node: TestedNode,
    amnezia_args: String,
    netstack_args: NetstackArgs,
    credentials_args: CredentialArgs,
    /// Pre-queried gateway node (used when --gateway-ip is specified)
    direct_gateway_node: Option<DirectoryNode>,
    /// Pre-queried exit gateway node (used when --exit-gateway-ip is specified for LP forwarding)
    exit_gateway_node: Option<DirectoryNode>,
    /// Localnet entry gateway info (used when --entry-gateway-identity is specified)
    localnet_entry: Option<TestedNodeDetails>,
    /// Localnet exit gateway info (used when --exit-gateway-identity is specified)
    localnet_exit: Option<TestedNodeDetails>,
    socks5_args: Socks5Args,
}

impl Probe {
    pub fn new(
        entrypoint: NodeIdentity,
        tested_node: TestedNode,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
        socks5_args: Socks5Args,
    ) -> Self {
        Self {
            entrypoint,
            tested_node,
            amnezia_args: "".into(),
            netstack_args,
            credentials_args,
            direct_gateway_node: None,
            exit_gateway_node: None,
            localnet_entry: None,
            localnet_exit: None,
            socks5_args,
        }
    }

    /// Create a probe with a pre-queried gateway node (for direct IP mode)
    pub fn new_with_gateway(
        entrypoint: NodeIdentity,
        tested_node: TestedNode,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
        gateway_node: DirectoryNode,
        socks5_args: Socks5Args,
    ) -> Self {
        Self {
            entrypoint,
            tested_node,
            amnezia_args: "".into(),
            netstack_args,
            credentials_args,
            direct_gateway_node: Some(gateway_node),
            exit_gateway_node: None,
            localnet_entry: None,
            localnet_exit: None,
            socks5_args,
        }
    }

    /// Create a probe with both entry and exit gateways pre-queried (for LP forwarding tests)
    pub fn new_with_gateways(
        entrypoint: NodeIdentity,
        tested_node: TestedNode,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
        entry_gateway_node: DirectoryNode,
        exit_gateway_node: DirectoryNode,
        socks5_args: Socks5Args,
    ) -> Self {
        Self {
            entrypoint,
            tested_node,
            amnezia_args: "".into(),
            netstack_args,
            credentials_args,
            direct_gateway_node: Some(entry_gateway_node),
            exit_gateway_node: Some(exit_gateway_node),
            localnet_entry: None,
            localnet_exit: None,
            socks5_args,
        }
    }

    /// Create a probe for localnet mode (no HTTP query needed)
    /// Uses identity + LP address directly from CLI args
    pub fn new_localnet(
        entry: TestedNodeDetails,
        exit: Option<TestedNodeDetails>,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
        socks5_args: Socks5Args,
    ) -> Self {
        let entrypoint = entry.identity;
        Self {
            entrypoint,
            tested_node: TestedNode::SameAsEntry,
            amnezia_args: "".into(),
            netstack_args,
            credentials_args,
            direct_gateway_node: None,
            exit_gateway_node: None,
            localnet_entry: Some(entry),
            localnet_exit: exit,
            socks5_args,
        }
    }

    pub fn with_amnezia(&mut self, args: &str) -> &Self {
        self.amnezia_args = args.to_string();
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn probe(
        self,
        directory: Option<NymApiDirectory>,
        nyxd_url: Url,
        ignore_egress_epoch_role: bool,
        only_wireguard: bool,
        only_lp_registration: bool,
        test_lp_wg: bool,
        min_mixnet_performance: Option<u8>,
        network_details: NymNetworkDetails,
    ) -> anyhow::Result<ProbeResult> {
        let tickets_materials = self.credentials_args.decode_attached_ticket_materials()?;

        let tested_entry = self.tested_node.is_same_as_entry();
        let (mixnet_entry_gateway_id, node_info) = self.lookup_gateway(&directory).await?;

        let storage = Ephemeral::default();

        // Connect to the mixnet via the entry gateway
        let disconnected_mixnet_client = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(mixnet_entry_gateway_id.to_string())
            .network_details(network_details.clone())
            .debug_config(helpers::mixnet_debug_config(
                min_mixnet_performance,
                ignore_egress_epoch_role,
            ))
            .with_forget_me(ForgetMe::new_all())
            .credentials_mode(true)
            .build()?;

        // in normal operation expects the ticket material to be provided as an argument
        let bandwidth_import = disconnected_mixnet_client.begin_bandwidth_import();
        import_bandwidth(bandwidth_import, tickets_materials).await?;

        let mixnet_client = Box::pin(disconnected_mixnet_client.connect_to_mixnet()).await;

        // Extract topology from the connected client (if successful) to reuse for SOCKS5 test
        let topology = match &mixnet_client {
            Ok(client) => client
                .read_current_route_provider()
                .await
                .map(|rp| rp.topology.clone()),
            Err(_) => None,
        };

        // Convert legacy flags to TestMode
        let has_exit = self.exit_gateway_node.is_some() || self.localnet_exit.is_some();
        let test_mode =
            TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

        self.do_probe_test(
            Some(mixnet_client),
            storage,
            mixnet_entry_gateway_id,
            node_info,
            directory.as_ref(),
            nyxd_url,
            tested_entry,
            test_mode,
            only_wireguard,
            false, // Not using mock ecash in regular probe mode
            network_details,
            topology,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn probe_run_locally(
        self,
        config_dir: &PathBuf,
        mnemonic: Option<&str>,
        directory: Option<NymApiDirectory>,
        nyxd_url: Url,
        ignore_egress_epoch_role: bool,
        only_wireguard: bool,
        only_lp_registration: bool,
        test_lp_wg: bool,
        min_mixnet_performance: Option<u8>,
        use_mock_ecash: bool,
        network_details: NymNetworkDetails,
    ) -> anyhow::Result<ProbeResult> {
        // Localnet mode - identity + LP address from CLI, no HTTP query
        // This path is used when --entry-gateway-identity is specified
        if let Some(entry_info) = &self.localnet_entry {
            info!("Using localnet mode with CLI-provided gateway identities");

            // Initialize storage (needed for credentials)
            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir)?;
            }
            let storage_paths = StoragePaths::new_from_dir(config_dir)?;
            let storage = storage_paths
                .initialise_default_persistent_storage()
                .await?;

            // For localnet, use entry as the test node (or exit if provided)
            let mixnet_entry_gateway_id = entry_info.identity;
            let node_info = if let Some(exit_info) = &self.localnet_exit {
                exit_info.clone()
            } else {
                entry_info.clone()
            };

            // Convert legacy flags to TestMode
            let has_exit = self.localnet_exit.is_some();
            let test_mode =
                TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

            return self
                .do_probe_test(
                    None,
                    storage,
                    mixnet_entry_gateway_id,
                    node_info,
                    directory.as_ref(),
                    nyxd_url,
                    false, // tested_entry
                    test_mode,
                    only_wireguard,
                    use_mock_ecash,
                    network_details,
                    None, // No topology (no mixnet client in localnet mode)
                )
                .await;
        }

        // If both gateways are pre-queried via --gateway-ip and --exit-gateway-ip,
        // skip mixnet setup entirely - we have all the data we need
        if self.direct_gateway_node.is_some() && self.exit_gateway_node.is_some() {
            let entry_node = if let Some(entry_node) = self.direct_gateway_node.as_ref() {
                entry_node
            } else {
                return Err(anyhow::anyhow!("Entry gateway node is missing"));
            };
            let exit_node = if let Some(exit_node) = self.exit_gateway_node.as_ref() {
                exit_node
            } else {
                return Err(anyhow::anyhow!("Exit gateway node is missing"));
            };

            // Initialize storage (needed for credentials)
            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir)?;
            }
            let storage_paths = StoragePaths::new_from_dir(config_dir)?;
            let storage = storage_paths
                .initialise_default_persistent_storage()
                .await?;

            // Get node details from pre-queried nodes
            let mixnet_entry_gateway_id = entry_node.identity();
            let node_info = exit_node.to_testable_node()?;

            // Convert legacy flags to TestMode (has_exit = true since we have exit_gateway_node)
            let test_mode =
                TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, true);

            return self
                .do_probe_test(
                    None,
                    storage,
                    mixnet_entry_gateway_id,
                    node_info,
                    directory.as_ref(),
                    nyxd_url,
                    false, // tested_entry
                    test_mode,
                    only_wireguard,
                    use_mock_ecash,
                    network_details,
                    None, // No topology (no mixnet client in direct gateway mode)
                )
                .await;
        }

        // If only testing LP registration, use the dedicated LP-only path
        // This skips mixnet setup entirely and allows testing local gateways
        if only_lp_registration {
            return self
                .probe_lp_only(config_dir, directory, nyxd_url, use_mock_ecash)
                .await;
        }

        let tested_entry = self.tested_node.is_same_as_entry();
        let (mixnet_entry_gateway_id, node_info) = self.lookup_gateway(&directory).await?;

        if config_dir.is_file() {
            bail!("provided configuration directory is a file");
        }

        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }

        let storage_paths = StoragePaths::new_from_dir(config_dir)?;
        let storage = storage_paths
            .initialise_default_persistent_storage()
            .await?;

        // Connect to the mixnet via the entry gateway, without forget-me flag so that gateway remembers client
        // and keeps its bandwidth between probe runs
        let disconnected_mixnet_client = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(mixnet_entry_gateway_id.to_string())
            .network_details(network_details.clone())
            .debug_config(helpers::mixnet_debug_config(
                min_mixnet_performance,
                ignore_egress_epoch_role,
            ))
            .credentials_mode(true)
            .build()?;

        let key_store = storage.key_store();
        let mut rng = OsRng;

        // WORKAROUND SINCE IT HASN'T MADE IT TO THE MONOREPO:
        if key_store.load_keys().await.is_err() {
            tracing::log::debug!("Generating new client keys");
            nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
        }

        let ticketbook_count = storage
            .credential_store()
            .get_ticketbooks_info()
            .await?
            .len();

        info!("Credential store contains {} ticketbooks", ticketbook_count);

        // Only acquire real bandwidth if not using mock ecash
        if ticketbook_count < 1 && !use_mock_ecash {
            let mnemonic = mnemonic.ok_or_else(|| {
                anyhow::anyhow!("mnemonic is required when not using mock ecash (--use-mock-ecash)")
            })?;
            for ticketbook_type in [
                TicketType::V1MixnetEntry,
                TicketType::V1WireguardEntry,
                TicketType::V1WireguardExit,
            ] {
                acquire_bandwidth(mnemonic, &disconnected_mixnet_client, ticketbook_type).await?;
            }
        } else if use_mock_ecash {
            info!("Using mock ecash mode - skipping bandwidth acquisition");
        }

        let mixnet_client = Box::pin(disconnected_mixnet_client.connect_to_mixnet()).await;

        // extract topology from the connected client (if any) to reuse for SOCKS5 test
        let topology = match &mixnet_client {
            Ok(client) => client
                .read_current_route_provider()
                .await
                .map(|rp| rp.topology.clone()),
            Err(_) => None,
        };

        // Convert legacy flags to TestMode
        let has_exit = self.exit_gateway_node.is_some() || self.localnet_exit.is_some();
        let test_mode =
            TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

        self.do_probe_test(
            Some(mixnet_client),
            storage,
            mixnet_entry_gateway_id,
            node_info,
            directory.as_ref(),
            nyxd_url,
            tested_entry,
            test_mode,
            only_wireguard,
            use_mock_ecash,
            network_details,
            topology,
        )
        .await
    }

    /// Probe LP registration only, skipping all mixnet tests
    /// This is useful for testing local dev gateways that aren't registered in nym-api
    pub async fn probe_lp_only(
        self,
        config_dir: &PathBuf,
        directory: Option<NymApiDirectory>,
        nyxd_url: Url,
        use_mock_ecash: bool,
    ) -> anyhow::Result<ProbeResult> {
        let tested_entry = self.tested_node.is_same_as_entry();
        let (mixnet_entry_gateway_id, node_info) = self.lookup_gateway(&directory).await?;

        if config_dir.is_file() {
            bail!("provided configuration directory is a file");
        }

        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }

        let storage_paths = StoragePaths::new_from_dir(config_dir)?;
        let storage = storage_paths
            .initialise_default_persistent_storage()
            .await?;

        let key_store = storage.key_store();
        let mut rng = OsRng;

        // Generate client keys if they don't exist
        if key_store.load_keys().await.is_err() {
            tracing::log::debug!("Generating new client keys");
            nym_client_core::init::generate_new_client_keys(&mut rng, key_store).await?;
        }

        // Check if node has LP address
        let Some(lp_data) = node_info.lp_data else {
            bail!("Gateway does not have LP data configured");
        };

        info!("Testing LP registration for gateway {}", node_info.identity);

        // Create bandwidth controller for credential preparation
        let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(
            &NymNetworkDetails::new_from_env(),
        )?;
        let client = nym_validator_client::nyxd::NyxdClient::connect(config, nyxd_url.as_str())?;
        let bw_controller = nym_bandwidth_controller::BandwidthController::new(
            storage.credential_store().clone(),
            client,
        );

        // Run LP registration probe
        let lp_outcome =
            lp_registration_probe(node_info.identity, lp_data, &bw_controller, use_mock_ecash)
                .await
                .unwrap_or_default();

        // Return result with only LP outcome
        Ok(ProbeResult {
            node: node_info.identity.to_string(),
            used_entry: mixnet_entry_gateway_id.to_string(),
            outcome: ProbeOutcome {
                as_entry: Entry::NotTested,
                as_exit: if tested_entry {
                    None
                } else {
                    Some(Exit::fail_to_connect())
                },
                wg: None,
                socks5: None,
                lp: Some(lp_outcome),
            },
        })
    }

    async fn test_socks5_if_possible(
        &self,
        network_details: NymNetworkDetails,
        network_requester_details: &Option<NetworkRequesterDetailsV1>,
        directory: &NymApiDirectory,
        topology: Option<NymTopology>,
    ) -> Option<Socks5ProbeResults> {
        if let Some(nr_details) = network_requester_details {
            match do_socks5_connectivity_test(
                &nr_details.address,
                network_details,
                directory,
                self.socks5_args.socks5_json_rpc_url_list.clone(),
                self.socks5_args.mixnet_client_timeout_sec,
                self.socks5_args.test_count,
                self.socks5_args.failure_count_cutoff,
                topology,
            )
            .await
            {
                Ok(results) => Some(results),
                Err(e) => {
                    error!("SOCKS5 test failed: {}", e);
                    None
                }
            }
        } else {
            info!("No NR available, skipping SOCKS5 tests");
            None
        }
    }

    pub async fn lookup_gateway(
        &self,
        directory: &Option<NymApiDirectory>,
    ) -> anyhow::Result<(NodeIdentity, TestedNodeDetails)> {
        // If we have a pre-queried gateway node (direct IP mode), use that
        if let Some(direct_node) = &self.direct_gateway_node {
            info!("Using pre-queried gateway node from direct IP query");
            let node_info = direct_node.to_testable_node()?;
            info!("connecting to entry gateway: {}", direct_node.identity());
            debug!(
                "authenticator version: {:?}",
                node_info.authenticator_version
            );
            return Ok((self.entrypoint, node_info));
        }

        // Otherwise, use the directory (original behavior)
        let directory = directory
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Directory is required when not using --gateway-ip"))?;

        // Setup the entry gateways
        let entry_gateway = directory.entry_gateway(&self.entrypoint)?;

        let node_info: TestedNodeDetails = match self.tested_node {
            TestedNode::Custom {
                identity: _,
                shares_entry: true,
            } => {
                debug!(
                    "testing node {} as both entry and exit",
                    entry_gateway.identity()
                );
                entry_gateway.to_testable_node()?
            }
            TestedNode::Custom {
                identity,
                shares_entry: false,
            } => {
                let node = directory.get_nym_node(identity)?;
                info!(
                    "testing node {} (via entry {})",
                    node.identity(),
                    entry_gateway.identity()
                );
                node.to_testable_node()?
            }
            TestedNode::SameAsEntry => entry_gateway.to_testable_node()?,
        };

        info!("connecting to entry gateway: {}", entry_gateway.identity());
        debug!(
            "authenticator version: {:?}",
            node_info.authenticator_version
        );

        Ok((self.entrypoint, node_info))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn do_probe_test<T>(
        &self,
        mixnet_client: Option<nym_sdk::Result<MixnetClient>>,
        storage: T,
        mixnet_entry_gateway_id: NodeIdentity,
        node_info: TestedNodeDetails,
        directory: Option<&NymApiDirectory>,
        nyxd_url: Url,
        tested_entry: bool,
        test_mode: TestMode,
        only_wireguard: bool,
        use_mock_ecash: bool,
        network_details: NymNetworkDetails,
        topology: Option<NymTopology>,
    ) -> anyhow::Result<ProbeResult>
    where
        T: MixnetClientStorage + Clone + 'static,
        <T::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    {
        let Some(directory) = directory else {
            bail!("You need to provide NYM API through environment")
        };
        // test_mode replaces the old only_lp_registration and test_lp_wg flags.
        // only_wireguard is kept separate as it controls ping behavior within Mixnet mode.
        let mut rng = rand::thread_rng();
        let mixnet_client = match mixnet_client {
            Some(Ok(mixnet_client)) => Some(mixnet_client),
            Some(Err(err)) => {
                error!("Failed to connect to mixnet: {err}");
                return Ok(ProbeResult {
                    node: node_info.identity.to_string(),
                    used_entry: mixnet_entry_gateway_id.to_string(),
                    outcome: ProbeOutcome {
                        as_entry: if tested_entry {
                            Entry::fail_to_connect()
                        } else {
                            Entry::EntryFailure
                        },
                        as_exit: None,
                        socks5: None,
                        wg: None,
                        lp: None,
                    },
                });
            }
            None => None,
        };

        // Determine if we should run ping tests:
        // - Only in Mixnet mode (LP modes don't use mixnet)
        // - And only if not --only-wireguard (which skips pings)
        let run_ping_tests = test_mode.needs_mixnet() && !only_wireguard;

        let (outcome, mixnet_client) = if let Some(mixnet_client) = mixnet_client {
            let nym_address = *mixnet_client.nym_address();
            let entry_gateway = nym_address.gateway().to_base58_string();

            info!("Successfully connected to entry gateway: {entry_gateway}");
            info!("Our nym address: {nym_address}");

            // Run ping tests if applicable
            let (outcome, mixnet_client) = if run_ping_tests {
                do_ping(
                    mixnet_client,
                    nym_address,
                    node_info.exit_router_address,
                    tested_entry,
                )
                .await
            } else {
                (
                    Ok(ProbeOutcome {
                        as_entry: if tested_entry {
                            Entry::success()
                        } else {
                            Entry::NotTested
                        },
                        as_exit: None,
                        socks5: None,
                        wg: None,
                        lp: None,
                    }),
                    mixnet_client,
                )
            };
            (outcome, Some(mixnet_client))
        } else if test_mode.uses_lp() && test_mode.tests_wireguard() {
            // LP modes (SingleHop/TwoHop) don't need mixnet client
            // Create default outcome and continue to LP-WG test below
            (
                Ok(ProbeOutcome {
                    as_entry: Entry::NotTested,
                    as_exit: None,
                    socks5: None,
                    wg: None,
                    lp: None,
                }),
                None,
            )
        } else {
            // For Mixnet mode, missing mixnet client is a failure
            (
                Ok(ProbeOutcome {
                    as_entry: if tested_entry {
                        Entry::fail_to_connect()
                    } else {
                        Entry::EntryFailure
                    },
                    as_exit: None,
                    socks5: None,
                    wg: None,
                    lp: None,
                }),
                None,
            )
        };

        let wg_outcome = if !test_mode.tests_wireguard() {
            // LpOnly mode: skip WireGuard test
            WgProbeResults::default()
        } else if test_mode.uses_lp() {
            // Test WireGuard via LP registration (nested session forwarding)
            info!("Testing WireGuard via LP registration (no mixnet)");

            // Create bandwidth controller for LP registration
            let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(
                &NymNetworkDetails::new_from_env(),
            )?;
            let client =
                nym_validator_client::nyxd::NyxdClient::connect(config, nyxd_url.as_str())?;
            let bw_controller = nym_bandwidth_controller::BandwidthController::new(
                storage.credential_store().clone(),
                client,
            );

            // Determine entry and exit gateways
            // Three modes for gateway resolution:
            // 1. direct_gateway_node/exit_gateway_node - from --gateway-ip (HTTP API query)
            // 2. localnet_entry/localnet_exit - from --entry-gateway-identity (CLI-only)
            // 3. directory lookup - original behavior for production
            let (entry_gateway, exit_gateway) = if let Some(exit_node) = &self.exit_gateway_node {
                // Both entry and exit gateways were pre-queried (direct IP mode)
                info!("Using pre-queried entry and exit gateways for LP forwarding test");
                let entry_node = self
                    .direct_gateway_node
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Entry gateway not available"))?;

                let entry_gateway = entry_node.to_testable_node()?;
                let exit_gateway = exit_node.to_testable_node()?;

                (entry_gateway, exit_gateway)
            } else if let Some(exit_localnet) = &self.localnet_exit {
                // Localnet mode: use CLI-provided identities and LP addresses
                info!("Using localnet entry and exit gateways for LP forwarding test");
                let entry_localnet = self.localnet_entry.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Entry gateway not available in localnet mode")
                })?;

                (entry_localnet.clone(), exit_localnet.clone())
            } else {
                // Original behavior: query from directory
                // The tested node is the exit
                let exit_gateway = node_info.clone();

                let entry_gateway_node = directory.entry_gateway(&mixnet_entry_gateway_id)?;
                let entry_gateway = entry_gateway_node.to_testable_node()?;

                (entry_gateway, exit_gateway)
            };

            wg_probe_lp(
                &entry_gateway,
                &exit_gateway,
                &bw_controller,
                use_mock_ecash,
                self.amnezia_args.clone(),
                self.netstack_args.clone(),
            )
            .await
            .unwrap_or_default()
        } else if let (Some(authenticator), Some(ip_address)) =
            (node_info.authenticator_address, node_info.ip_address)
        {
            let mixnet_client = if let Some(mixnet_client) = mixnet_client {
                mixnet_client
            } else {
                bail!(
                    "Mixnet client is required for authenticator WireGuard probe, run in LP mode instead"
                );
            };

            let nym_address = *mixnet_client.nym_address();
            // Start the mixnet listener that the auth clients use to receive messages.
            let mixnet_listener_task =
                AuthClientMixnetListener::new(mixnet_client, CancellationToken::new()).start();

            let auth_client = AuthenticatorClient::new(
                mixnet_listener_task.subscribe(),
                mixnet_listener_task.mixnet_sender(),
                nym_address,
                authenticator,
                node_info.authenticator_version,
                Arc::new(KeyPair::new(&mut rng)),
                ip_address,
            );
            let config =
                nym_validator_client::nyxd::Config::try_from_nym_network_details(&network_details)?;
            let client =
                nym_validator_client::nyxd::NyxdClient::connect(config, nyxd_url.as_str())?;
            let bw_controller = nym_bandwidth_controller::BandwidthController::new(
                storage.credential_store().clone(),
                client,
            );
            let (wg_ticket_type, credential_provider) = if tested_entry {
                (
                    TicketType::V1WireguardEntry,
                    nym_address.gateway().to_bytes(),
                )
            } else {
                (TicketType::V1WireguardExit, node_info.identity.to_bytes())
            };

            let credential = bw_controller
                .prepare_ecash_ticket(wg_ticket_type, credential_provider, 1)
                .await?
                .data;

            let outcome = wg_probe(
                auth_client,
                ip_address,
                node_info.authenticator_version,
                self.amnezia_args.clone(),
                self.netstack_args.clone(),
                credential,
            )
            .await
            .unwrap_or_default();

            mixnet_listener_task.stop().await;

            outcome
        } else {
            WgProbeResults::default()
        };

        // Test LP registration if node has LP address
        let lp_outcome = if let Some(lp_data) = node_info.lp_data {
            info!("Node has LP data, testing LP registration...");

            // Prepare bandwidth credential for LP registration
            let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(
                &NymNetworkDetails::new_from_env(),
            )?;
            let client =
                nym_validator_client::nyxd::NyxdClient::connect(config, nyxd_url.as_str())?;
            let bw_controller = nym_bandwidth_controller::BandwidthController::new(
                storage.credential_store().clone(),
                client,
            );

            let outcome =
                lp_registration_probe(node_info.identity, lp_data, &bw_controller, use_mock_ecash)
                    .await
                    .unwrap_or_default();

            Some(outcome)
        } else {
            info!("Node does not have LP address, skipping LP registration test");
            None
        };

        // test failure doesn't stop further tests
        let socks5_outcome = self
            .test_socks5_if_possible(
                network_details,
                &node_info.network_requester_details,
                directory,
                topology,
            )
            .await;

        // Disconnect the mixnet client gracefully
        outcome.map(|mut outcome| {
            outcome.wg = Some(wg_outcome);
            outcome.lp = lp_outcome;
            outcome.socks5 = socks5_outcome;
            ProbeResult {
                node: node_info.identity.to_string(),
                used_entry: mixnet_entry_gateway_id.to_string(),
                outcome,
            }
        })
    }
}
