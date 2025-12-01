// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::Duration,
};

use crate::types::Entry;
use anyhow::bail;
use base64::{Engine as _, engine::general_purpose};
use bytes::BytesMut;
use clap::Args;
use futures::StreamExt;
use nym_authenticator_client::{AuthClientMixnetListener, AuthenticatorClient};
use nym_authenticator_requests::{
    AuthenticatorVersion, client_message::ClientMessage, response::AuthenticatorResponse, v2, v3,
    v4, v5, v6,
};
use nym_client_core::config::ForgetMe;
use nym_config::defaults::{
    NymNetworkDetails,
    mixnet_vpn::{NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6},
};
use nym_connection_monitor::self_ping_and_wait;
use nym_credentials_interface::{CredentialSpendingData, TicketType};
use nym_crypto::asymmetric::x25519::KeyPair;
use nym_ip_packet_client::IprClientConnect;
use nym_ip_packet_requests::{
    IpPair,
    codec::MultiIpPacketCodec,
    v8::response::{
        ControlResponse, DataResponse, InfoLevel, IpPacketResponse, IpPacketResponseData,
    },
};
use nym_sdk::mixnet::{
    CredentialStorage, Ephemeral, KeyStore, MixnetClient, MixnetClientBuilder, MixnetClientStorage,
    NodeIdentity, Recipient, ReconstructedMessage, StoragePaths,
};
use rand::rngs::OsRng;
use std::path::PathBuf;

use tokio_util::{codec::Decoder, sync::CancellationToken};
use tracing::*;
use types::WgProbeResults;
use url::Url;

use crate::{
    icmp::{check_for_icmp_beacon_reply, icmp_identifier, send_ping_v4, send_ping_v6},
    types::Exit,
};


mod bandwidth_helpers;
mod common;
mod icmp;
pub mod mode;
mod netstack;
pub mod nodes;
mod types;

use crate::bandwidth_helpers::{acquire_bandwidth, import_bandwidth};
use crate::nodes::{DirectoryNode, NymApiDirectory};
use nym_node_status_client::models::AttachedTicketMaterials;
pub use mode::TestMode;
pub use types::{IpPingReplies, ProbeOutcome, ProbeResult};

#[derive(Args, Clone)]
pub struct NetstackArgs {
    #[arg(long, default_value_t = 180)]
    netstack_download_timeout_sec: u64,

    #[arg(long, default_value_t = 30)]
    metadata_timeout_sec: u64,

    #[arg(long, default_value = "1.1.1.1")]
    netstack_v4_dns: String,

    #[arg(long, default_value = "2606:4700:4700::1111")]
    netstack_v6_dns: String,

    #[arg(long, default_value_t = 5)]
    netstack_num_ping: u8,

    #[arg(long, default_value_t = 3)]
    netstack_send_timeout_sec: u64,

    #[arg(long, default_value_t = 3)]
    netstack_recv_timeout_sec: u64,

    #[arg(long, default_values_t = vec!["nym.com".to_string()])]
    netstack_ping_hosts_v4: Vec<String>,

    #[arg(long, default_values_t = vec!["1.1.1.1".to_string()])]
    netstack_ping_ips_v4: Vec<String>,

    #[arg(long, default_values_t = vec!["cloudflare.com".to_string()])]
    netstack_ping_hosts_v6: Vec<String>,

    #[arg(long, default_values_t = vec!["2001:4860:4860::8888".to_string(), "2606:4700:4700::1111".to_string(), "2620:fe::fe".to_string()])]
    netstack_ping_ips_v6: Vec<String>,
}

#[derive(Args)]
pub struct CredentialArgs {
    #[arg(long)]
    ticket_materials: Option<String>,

    #[arg(long, default_value_t = 1)]
    ticket_materials_revision: u8,
}

impl CredentialArgs {
    fn decode_attached_ticket_materials(&self) -> anyhow::Result<AttachedTicketMaterials> {
        let ticket_materials = self
            .ticket_materials
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ticket_materials is required"))?
            .clone();

        Ok(AttachedTicketMaterials::from_serialised_string(
            ticket_materials,
            self.ticket_materials_revision,
        )?)
    }
}

#[derive(Default, Debug)]
pub enum TestedNode {
    #[default]
    SameAsEntry,
    Custom {
        identity: NodeIdentity,
    },
}

impl TestedNode {
    pub fn is_same_as_entry(&self) -> bool {
        matches!(self, TestedNode::SameAsEntry)
    }
}

#[derive(Debug, Clone)]
pub struct TestedNodeDetails {
    identity: NodeIdentity,
    exit_router_address: Option<Recipient>,
    authenticator_address: Option<Recipient>,
    authenticator_version: AuthenticatorVersion,
    ip_address: Option<IpAddr>,
    lp_address: Option<std::net::SocketAddr>,
}

impl TestedNodeDetails {
    /// Create from CLI args (localnet mode - no HTTP query needed)
    /// Only identity and LP address are required; other fields are None/default.
    pub fn from_cli(identity: NodeIdentity, lp_address: std::net::SocketAddr) -> Self {
        Self {
            identity,
            ip_address: Some(lp_address.ip()),
            lp_address: Some(lp_address),
            // These are None in localnet mode - only needed for mixnet/authenticator
            exit_router_address: None,
            authenticator_address: None,
            authenticator_version: AuthenticatorVersion::UNKNOWN,
        }
    }

    /// Check if this node has sufficient info for LP testing
    pub fn can_test_lp(&self) -> bool {
        self.lp_address.is_some()
    }

    /// Check if this node has sufficient info for mixnet testing
    pub fn can_test_mixnet(&self) -> bool {
        self.exit_router_address.is_some() || self.authenticator_address.is_some()
    }
}

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
}

impl Probe {
    pub fn new(
        entrypoint: NodeIdentity,
        tested_node: TestedNode,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
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
        }
    }

    /// Create a probe with a pre-queried gateway node (for direct IP mode)
    pub fn new_with_gateway(
        entrypoint: NodeIdentity,
        tested_node: TestedNode,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
        gateway_node: DirectoryNode,
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
        }
    }

    /// Create a probe for localnet mode (no HTTP query needed)
    /// Uses identity + LP address directly from CLI args
    pub fn new_localnet(
        entry: TestedNodeDetails,
        exit: Option<TestedNodeDetails>,
        netstack_args: NetstackArgs,
        credentials_args: CredentialArgs,
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
        }
    }

    pub fn with_amnezia(&mut self, args: &str) -> &Self {
        self.amnezia_args = args.to_string();
        self
    }

    pub async fn probe(
        self,
        directory: Option<NymApiDirectory>,
        nyxd_url: Url,
        ignore_egress_epoch_role: bool,
        only_wireguard: bool,
        only_lp_registration: bool,
        test_lp_wg: bool,
        min_mixnet_performance: Option<u8>,
    ) -> anyhow::Result<ProbeResult> {
        let tickets_materials = self.credentials_args.decode_attached_ticket_materials()?;

        let tested_entry = self.tested_node.is_same_as_entry();
        let (mixnet_entry_gateway_id, node_info) = self.lookup_gateway(&directory).await?;

        let storage = Ephemeral::default();

        // Connect to the mixnet via the entry gateway
        let disconnected_mixnet_client = MixnetClientBuilder::new_with_storage(storage.clone())
            .request_gateway(mixnet_entry_gateway_id.to_string())
            .network_details(NymNetworkDetails::new_from_env())
            .debug_config(mixnet_debug_config(
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

        // Convert legacy flags to TestMode
        let has_exit = self.exit_gateway_node.is_some() || self.localnet_exit.is_some();
        let test_mode = TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

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
    ) -> anyhow::Result<ProbeResult> {
        // AIDEV-NOTE: Localnet mode - identity + LP address from CLI, no HTTP query
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
            let test_mode = TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

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
                )
                .await;
        }

        // If both gateways are pre-queried via --gateway-ip and --exit-gateway-ip,
        // skip mixnet setup entirely - we have all the data we need
        if self.direct_gateway_node.is_some() && self.exit_gateway_node.is_some() {
            let entry_node = self.direct_gateway_node.as_ref().unwrap();
            let exit_node = self.exit_gateway_node.as_ref().unwrap();

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
            let test_mode = TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, true);

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
            .network_details(NymNetworkDetails::new_from_env())
            .debug_config(mixnet_debug_config(
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

        // Convert legacy flags to TestMode
        let has_exit = self.exit_gateway_node.is_some() || self.localnet_exit.is_some();
        let test_mode = TestMode::from_flags(only_wireguard, only_lp_registration, test_lp_wg, has_exit);

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
        let (lp_address, ip_address) = match (node_info.lp_address, node_info.ip_address) {
            (Some(lp_addr), Some(ip_addr)) => (lp_addr, ip_addr),
            _ => {
                bail!("Gateway does not have LP address configured");
            }
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
        let lp_outcome = lp_registration_probe(
            node_info.identity,
            lp_address,
            ip_address,
            &bw_controller,
            use_mock_ecash,
        )
        .await
        .unwrap_or_default();

        // Return result with only LP outcome
        Ok(ProbeResult {
            node: node_info.identity.to_string(),
            used_entry: mixnet_entry_gateway_id.to_string(),
            outcome: types::ProbeOutcome {
                as_entry: types::Entry::NotTested,
                as_exit: if tested_entry {
                    None
                } else {
                    Some(types::Exit::fail_to_connect())
                },
                wg: None,
                lp: Some(lp_outcome),
            },
        })
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
            TestedNode::Custom { identity } => {
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
    ) -> anyhow::Result<ProbeResult>
    where
        T: MixnetClientStorage + Clone + 'static,
        <T::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    {
        // AIDEV-NOTE: test_mode replaces the old only_lp_registration and test_lp_wg flags.
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
            (Ok(ProbeOutcome {
                as_entry: Entry::NotTested,
                as_exit: None,
                wg: None,
                lp: None,
            }), None)
        } else {
            // For Mixnet mode, missing mixnet client is a failure
            (Ok(ProbeOutcome {
                as_entry: if tested_entry {
                    Entry::fail_to_connect()
                } else {
                    Entry::EntryFailure
                },
                as_exit: None,
                wg: None,
                lp: None,
            }), None)
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
            } else {
                // Original behavior: query from directory
                // The tested node is the exit
                let exit_gateway = node_info.clone();

                let directory = directory
                    .ok_or_else(|| anyhow::anyhow!("Directory is required for LP-WG test mode"))?;
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
            let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(
                &NymNetworkDetails::new_from_env(),
            )?;
            let client =
                nym_validator_client::nyxd::NyxdClient::connect(config, nyxd_url.as_str())?;
            let bw_controller = nym_bandwidth_controller::BandwidthController::new(
                storage.credential_store().clone(),
                client,
            );
            let credential = bw_controller
                .prepare_ecash_ticket(
                    TicketType::V1WireguardEntry,
                    nym_address.gateway().to_bytes(),
                    1,
                )
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
        let lp_outcome = if let (Some(lp_address), Some(ip_address)) =
            (node_info.lp_address, node_info.ip_address)
        {
            info!("Node has LP address, testing LP registration...");

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

            let outcome = lp_registration_probe(
                node_info.identity,
                lp_address,
                ip_address,
                &bw_controller,
                use_mock_ecash,
            )
            .await
            .unwrap_or_default();

            Some(outcome)
        } else {
            info!("Node does not have LP address, skipping LP registration test");
            None
        };

        // Disconnect the mixnet client gracefully
        outcome.map(|mut outcome| {
            outcome.wg = Some(wg_outcome);
            outcome.lp = lp_outcome;
            ProbeResult {
                node: node_info.identity.to_string(),
                used_entry: mixnet_entry_gateway_id.to_string(),
                outcome,
            }
        })
    }
}

async fn wg_probe(
    mut auth_client: AuthenticatorClient,
    gateway_ip: IpAddr,
    auth_version: AuthenticatorVersion,
    awg_args: String,
    netstack_args: NetstackArgs,
    // TODO: update type
    credential: CredentialSpendingData,
) -> anyhow::Result<WgProbeResults> {
    info!("attempting to use authenticator version {auth_version:?}");

    let mut rng = rand::thread_rng();

    // that's a long conversion chain
    // (it should be simplified later...)
    // nym x25519 -> dalek x25519 -> wireguard wrapper x25519
    let private_key = nym_crypto::asymmetric::encryption::PrivateKey::new(&mut rng);
    let public_key = private_key.public_key();

    let authenticator_pub_key = public_key.inner().into();
    let init_message = match auth_version {
        AuthenticatorVersion::V2 => ClientMessage::Initial(Box::new(
            v2::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V3 => ClientMessage::Initial(Box::new(
            v3::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V4 => ClientMessage::Initial(Box::new(
            v4::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V5 => ClientMessage::Initial(Box::new(
            v5::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V6 => ClientMessage::Initial(Box::new(
            v6::registration::InitMessage::new(authenticator_pub_key),
        )),
        AuthenticatorVersion::V1 | AuthenticatorVersion::UNKNOWN => bail!("unknown version number"),
    };

    let mut wg_outcome = WgProbeResults::default();

    info!(
        "connecting to authenticator: {}...",
        auth_client.auth_recipient
    );
    let response = auth_client
        .send_and_wait_for_response(&init_message)
        .await?;

    let registered_data = match response {
        AuthenticatorResponse::PendingRegistration(pending_registration_response) => {
            // Unwrap since we have already checked that we have the keypair.
            debug!("Verifying data");
            pending_registration_response.verify(&private_key)?;

            let credential = credential
                .try_into()
                .inspect_err(|err| error!("invalid zk-nym data: {err}"))
                .ok();

            let finalized_message =
                pending_registration_response.finalise_registration(&private_key, credential);
            let client_message = ClientMessage::Final(finalized_message);

            let response = auth_client
                .send_and_wait_for_response(&client_message)
                .await?;
            let AuthenticatorResponse::Registered(registered_response) = response else {
                bail!("Unexpected response");
            };
            registered_response
        }
        AuthenticatorResponse::Registered(registered_response) => registered_response,
        _ => bail!("Unexpected response"),
    };

    let peer_public = registered_data.pub_key().inner();
    let static_private = x25519_dalek::StaticSecret::from(private_key.to_bytes());
    let public_key_bs64 = general_purpose::STANDARD.encode(peer_public.as_bytes());
    let private_key_hex = hex::encode(static_private.to_bytes());
    let public_key_hex = hex::encode(peer_public.as_bytes());

    info!("WG connection details");
    info!("Peer public key: {}", public_key_bs64);
    info!(
        "ips {}(v4) {}(v6), port {}",
        registered_data.private_ips().ipv4,
        registered_data.private_ips().ipv6,
        registered_data.wg_port(),
    );

    let wg_endpoint = format!("{gateway_ip}:{}", registered_data.wg_port());

    info!("Successfully registered with the gateway");

    wg_outcome.can_register = true;

    // Run tunnel connectivity tests using shared helper
    let tunnel_config = common::WgTunnelConfig::new(
        registered_data.private_ips().ipv4.to_string(),
        registered_data.private_ips().ipv6.to_string(),
        private_key_hex,
        public_key_hex,
        wg_endpoint,
    );

    common::run_tunnel_tests(&tunnel_config, &netstack_args, &awg_args, &mut wg_outcome);

    Ok(wg_outcome)
}

async fn lp_registration_probe<St>(
    gateway_identity: NodeIdentity,
    gateway_lp_address: std::net::SocketAddr,
    gateway_ip: IpAddr,
    bandwidth_controller: &nym_bandwidth_controller::BandwidthController<
        nym_validator_client::nyxd::NyxdClient<nym_validator_client::HttpRpcClient>,
        St,
    >,
    use_mock_ecash: bool,
) -> anyhow::Result<types::LpProbeResults>
where
    St: nym_sdk::mixnet::CredentialStorage + Clone + Send + Sync + 'static,
    <St as nym_sdk::mixnet::CredentialStorage>::StorageError: Send + Sync,
{
    use nym_crypto::asymmetric::ed25519;
    use nym_registration_client::LpRegistrationClient;

    info!(
        "Starting LP registration probe for gateway at {}",
        gateway_lp_address
    );

    let mut lp_outcome = types::LpProbeResults::default();

    // Generate Ed25519 keypair for this connection (X25519 will be derived internally by LP)
    let mut rng = rand::thread_rng();
    let client_ed25519_keypair = std::sync::Arc::new(ed25519::KeyPair::new(&mut rng));

    // Create LP registration client (uses Ed25519 keys directly, derives X25519 internally)
    let mut client = LpRegistrationClient::new_with_default_psk(
        client_ed25519_keypair,
        gateway_identity,
        gateway_lp_address,
        gateway_ip,
    );

    // Step 1: Perform handshake (connection is implicit in packet-per-connection model)
    // AIDEV-NOTE: LpRegistrationClient uses packet-per-connection model - connect() is gone,
    // connection is established during handshake and registration automatically.
    info!("Performing LP handshake at {}...", gateway_lp_address);
    match client.perform_handshake().await {
        Ok(_) => {
            info!("LP handshake completed successfully");
            lp_outcome.can_connect = true; // Connection succeeded if handshake succeeded
            lp_outcome.can_handshake = true;
        }
        Err(e) => {
            let error_msg = format!("LP handshake failed: {}", e);
            error!("{}", error_msg);
            lp_outcome.error = Some(error_msg);
            return Ok(lp_outcome);
        }
    }

    // Step 2: Register with gateway (send request + receive response in one call)
    info!("Sending LP registration request...");

    // Generate WireGuard keypair for dVPN registration
    let mut rng = rand::thread_rng();
    let wg_keypair = nym_crypto::asymmetric::x25519::KeyPair::new(&mut rng);

    // Convert gateway identity to ed25519 public key
    let gateway_ed25519_pubkey = match nym_crypto::asymmetric::ed25519::PublicKey::from_bytes(
        &gateway_identity.to_bytes(),
    ) {
        Ok(key) => key,
        Err(e) => {
            let error_msg = format!("Failed to convert gateway identity: {}", e);
            error!("{}", error_msg);
            lp_outcome.error = Some(error_msg);
            return Ok(lp_outcome);
        }
    };

    // Register using the new packet-per-connection API (returns GatewayData directly)
    let ticket_type = TicketType::V1WireguardEntry;
    let gateway_data = if use_mock_ecash {
        info!("Using mock ecash credential for LP registration");
        let credential = crate::bandwidth_helpers::create_dummy_credential(
            &gateway_ed25519_pubkey.to_bytes(),
            ticket_type,
        );

        match client.register_with_credential(&wg_keypair, credential, ticket_type).await {
            Ok(data) => data,
            Err(e) => {
                let error_msg = format!("LP registration failed (mock ecash): {}", e);
                error!("{}", error_msg);
                lp_outcome.error = Some(error_msg);
                return Ok(lp_outcome);
            }
        }
    } else {
        info!("Using real bandwidth controller for LP registration");
        match client
            .register(&wg_keypair, &gateway_ed25519_pubkey, bandwidth_controller, ticket_type)
            .await
        {
            Ok(data) => data,
            Err(e) => {
                let error_msg = format!("LP registration failed: {}", e);
                error!("{}", error_msg);
                lp_outcome.error = Some(error_msg);
                return Ok(lp_outcome);
            }
        }
    };

    info!("LP registration successful! Received gateway data:");
    info!("  - Gateway public key: {:?}", gateway_data.public_key);
    info!("  - Private IPv4: {}", gateway_data.private_ipv4);
    info!("  - Private IPv6: {}", gateway_data.private_ipv6);
    info!("  - Endpoint: {}", gateway_data.endpoint);
    lp_outcome.can_register = true;

    Ok(lp_outcome)
}

/// LP-based WireGuard probe: Tests LP nested session registration + WireGuard tunnel connectivity
///
/// This function tests the full VPN flow using LP registration instead of mixnet+authenticator:
/// 1. Connects to entry gateway (outer LP session)
/// 2. Registers with exit gateway via entry forwarding (nested LP session)
/// 3. Receives WireGuard configuration from both gateways
/// 4. Tests WireGuard tunnel connectivity (IPv4/IPv6)
///
/// This validates that IP hiding works (exit sees entry IP, not client IP) and that the
/// full VPN tunnel operates correctly after LP registration.
async fn wg_probe_lp<St>(
    entry_gateway: &TestedNodeDetails,
    exit_gateway: &TestedNodeDetails,
    bandwidth_controller: &nym_bandwidth_controller::BandwidthController<
        nym_validator_client::nyxd::NyxdClient<nym_validator_client::HttpRpcClient>,
        St,
    >,
    use_mock_ecash: bool,
    awg_args: String,
    netstack_args: NetstackArgs,
) -> anyhow::Result<WgProbeResults>
where
    St: nym_sdk::mixnet::CredentialStorage + Clone + Send + Sync + 'static,
    <St as nym_sdk::mixnet::CredentialStorage>::StorageError: Send + Sync,
{
    use nym_crypto::asymmetric::{ed25519, x25519};
    use nym_registration_client::{LpRegistrationClient, NestedLpSession};

    info!("Starting LP-based WireGuard probe (entry→exit via forwarding)");

    let mut wg_outcome = WgProbeResults::default();

    // Validate that both gateways have required information
    let entry_lp_address = entry_gateway
        .lp_address
        .ok_or_else(|| anyhow::anyhow!("Entry gateway missing LP address"))?;
    let exit_lp_address = exit_gateway
        .lp_address
        .ok_or_else(|| anyhow::anyhow!("Exit gateway missing LP address"))?;
    let entry_ip = entry_gateway
        .ip_address
        .ok_or_else(|| anyhow::anyhow!("Entry gateway missing IP address"))?;
    let exit_ip = exit_gateway
        .ip_address
        .ok_or_else(|| anyhow::anyhow!("Exit gateway missing IP address"))?;

    // Generate Ed25519 keypairs for LP protocol
    let mut rng = rand::thread_rng();
    let entry_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut rng));
    let exit_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut rng));

    // Generate WireGuard keypairs for VPN registration
    let entry_wg_keypair = x25519::KeyPair::new(&mut rng);
    let exit_wg_keypair = x25519::KeyPair::new(&mut rng);

    // STEP 1: Establish outer LP session with entry gateway
    // AIDEV-NOTE: LpRegistrationClient uses packet-per-connection model - connect() is gone,
    // connection is established automatically during handshake.
    info!("Establishing outer LP session with entry gateway...");
    let mut entry_client = LpRegistrationClient::new_with_default_psk(
        entry_lp_keypair,
        entry_gateway.identity,
        entry_lp_address,
        entry_ip,
    );

    // Perform handshake with entry gateway (connection is implicit)
    if let Err(e) = entry_client.perform_handshake().await {
        error!("Failed to handshake with entry gateway: {}", e);
        return Ok(wg_outcome);
    }
    info!("Outer LP session with entry gateway established");

    // STEP 2: Use nested session to register with exit gateway via forwarding
    info!("Registering with exit gateway via entry forwarding...");
    let mut nested_session = NestedLpSession::new(
        exit_gateway.identity.to_bytes(),
        exit_lp_address.to_string(),
        exit_lp_keypair,
        ed25519::PublicKey::from_bytes(&exit_gateway.identity.to_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid exit gateway identity: {}", e))?,
    );

    // Convert exit gateway identity to ed25519 public key for registration
    let exit_gateway_pubkey = ed25519::PublicKey::from_bytes(&exit_gateway.identity.to_bytes())
        .map_err(|e| anyhow::anyhow!("Invalid exit gateway identity: {}", e))?;

    // Perform handshake and registration with exit gateway via forwarding
    if use_mock_ecash {
        info!("Note: Using mock ecash mode - gateways must be started with --lp-use-mock-ecash");
    }
    let exit_gateway_data = match nested_session
        .handshake_and_register(
            &mut entry_client,
            &exit_wg_keypair,
            &exit_gateway_pubkey,
            bandwidth_controller,
            TicketType::V1WireguardExit,
            exit_ip,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to register with exit gateway: {}", e);
            return Ok(wg_outcome);
        }
    };
    info!("Exit gateway registration successful via forwarding");

    // STEP 3: Register with entry gateway
    info!("Registering with entry gateway...");
    let entry_gateway_pubkey =
        ed25519::PublicKey::from_bytes(&entry_gateway.identity.to_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid entry gateway identity: {}", e))?;

    // Use packet-per-connection register() which returns GatewayData directly
    let _entry_gateway_data = match entry_client
        .register(
            &entry_wg_keypair,
            &entry_gateway_pubkey,
            bandwidth_controller,
            TicketType::V1WireguardEntry,
        )
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to register with entry gateway: {}", e);
            return Ok(wg_outcome);
        }
    };
    info!("Entry gateway registration successful");

    info!("LP registration successful for both gateways!");
    wg_outcome.can_register = true;

    // STEP 4: Test WireGuard tunnels using exit gateway configuration
    // Convert keys to hex for netstack
    let private_key_hex = hex::encode(exit_wg_keypair.private_key().to_bytes());
    let public_key_hex = hex::encode(exit_gateway_data.public_key.to_bytes());

    // Build WireGuard endpoint address
    let wg_endpoint = format!("{}:{}", exit_ip, exit_gateway_data.endpoint.port());

    info!("Exit WireGuard configuration:");
    info!("  Private IPv4: {}", exit_gateway_data.private_ipv4);
    info!("  Private IPv6: {}", exit_gateway_data.private_ipv6);
    info!("  Endpoint: {}", wg_endpoint);

    // Run tunnel connectivity tests using shared helper
    let tunnel_config = common::WgTunnelConfig::new(
        exit_gateway_data.private_ipv4.to_string(),
        exit_gateway_data.private_ipv6.to_string(),
        private_key_hex,
        public_key_hex,
        wg_endpoint,
    );

    common::run_tunnel_tests(&tunnel_config, &netstack_args, &awg_args, &mut wg_outcome);

    info!("LP-based WireGuard probe completed");
    Ok(wg_outcome)
}

fn mixnet_debug_config(
    min_gateway_performance: Option<u8>,
    ignore_egress_epoch_role: bool,
) -> nym_client_core::config::DebugConfig {
    let mut debug_config = nym_client_core::config::DebugConfig::default();
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = true;
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;
    if let Some(minimum_gateway_performance) = min_gateway_performance {
        debug_config.topology.minimum_gateway_performance = minimum_gateway_performance;
    }
    if ignore_egress_epoch_role {
        debug_config.topology.ignore_egress_epoch_role = ignore_egress_epoch_role;
    }

    debug_config
}

async fn do_ping(
    mut mixnet_client: MixnetClient,
    our_address: Recipient,
    exit_router_address: Option<Recipient>,
    tested_entry: bool,
) -> (anyhow::Result<ProbeOutcome>, MixnetClient) {
    let entry = do_ping_entry(&mut mixnet_client, our_address, tested_entry).await;

    let (exit_result, mixnet_client) = if let Some(exit_router_address) = exit_router_address {
        let (maybe_ip_pair, mut mixnet_client) =
            connect_exit(mixnet_client, exit_router_address).await;
        match maybe_ip_pair {
            Some(ip_pair) => (
                do_ping_exit(&mut mixnet_client, ip_pair, exit_router_address).await,
                mixnet_client,
            ),
            None => (Ok(Some(Exit::fail_to_connect())), mixnet_client),
        }
    } else {
        (Ok(None), mixnet_client)
    };

    (
        exit_result.map(|exit| ProbeOutcome {
            as_entry: entry,
            as_exit: exit,
            wg: None,
            lp: None,
        }),
        mixnet_client,
    )
}

async fn do_ping_entry(
    mixnet_client: &mut MixnetClient,
    our_address: Recipient,
    tested_entry: bool,
) -> Entry {
    // Step 1: confirm that the entry gateway is routing our mixnet traffic
    info!("Sending mixnet ping to ourselves to verify mixnet connection");

    if self_ping_and_wait(our_address, mixnet_client)
        .await
        .is_err()
    {
        return if tested_entry {
            Entry::fail_to_connect()
        } else {
            Entry::EntryFailure
        };
    }
    info!("Successfully mixnet pinged ourselves");

    if tested_entry {
        Entry::success()
    } else {
        Entry::NotTested
    }
}

async fn connect_exit(
    mixnet_client: MixnetClient,
    exit_router_address: Recipient,
) -> (Option<IpPair>, MixnetClient) {
    // Step 2: connect to the exit gateway
    info!(
        "Connecting to exit gateway: {}",
        exit_router_address.gateway().to_base58_string()
    );
    // The IPR supports cancellation, but it's unused in the gateway probe
    let cancel_token = CancellationToken::new();
    let mut ipr_client = IprClientConnect::new(mixnet_client, cancel_token);

    let maybe_ip_pair = ipr_client.connect(exit_router_address).await;
    let mixnet_client = ipr_client.into_mixnet_client();

    if let Ok(our_ips) = maybe_ip_pair {
        info!("Successfully connected to exit gateway");
        info!("Using mixnet VPN IP addresses: {our_ips}");
        (Some(our_ips), mixnet_client)
    } else {
        (None, mixnet_client)
    }
}

async fn do_ping_exit(
    mixnet_client: &mut MixnetClient,
    our_ips: IpPair,
    exit_router_address: Recipient,
) -> anyhow::Result<Option<Exit>> {
    // Step 3: perform ICMP connectivity checks for the exit gateway
    send_icmp_pings(mixnet_client, our_ips, exit_router_address).await?;
    listen_for_icmp_ping_replies(mixnet_client, our_ips).await
}

async fn send_icmp_pings(
    mixnet_client: &MixnetClient,
    our_ips: IpPair,
    exit_router_address: Recipient,
) -> anyhow::Result<()> {
    // ipv4 addresses for testing
    let ipr_tun_ip_v4 = NYM_TUN_DEVICE_ADDRESS_V4;
    let external_ip_v4 = Ipv4Addr::new(8, 8, 8, 8);

    // ipv6 addresses for testing
    let ipr_tun_ip_v6 = NYM_TUN_DEVICE_ADDRESS_V6;
    let external_ip_v6 = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);

    info!(
        "Sending ICMP echo requests to: {ipr_tun_ip_v4}, {ipr_tun_ip_v6}, {external_ip_v4}, {external_ip_v6}"
    );

    // send ipv4 pings
    for ii in 0..10 {
        send_ping_v4(
            mixnet_client,
            our_ips,
            ii,
            ipr_tun_ip_v4,
            exit_router_address,
        )
        .await?;
        send_ping_v4(
            mixnet_client,
            our_ips,
            ii,
            external_ip_v4,
            exit_router_address,
        )
        .await?;
    }

    // send ipv6 pings
    for ii in 0..10 {
        send_ping_v6(
            mixnet_client,
            our_ips,
            ii,
            ipr_tun_ip_v6,
            exit_router_address,
        )
        .await?;
        send_ping_v6(
            mixnet_client,
            our_ips,
            ii,
            external_ip_v6,
            exit_router_address,
        )
        .await?;
    }
    Ok(())
}

async fn listen_for_icmp_ping_replies(
    mixnet_client: &mut MixnetClient,
    our_ips: IpPair,
) -> anyhow::Result<Option<Exit>> {
    let mut multi_ip_packet_decoder = MultiIpPacketCodec::new();
    let mut registered_replies = IpPingReplies::new();

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                info!("Finished waiting for ICMP echo reply from exit gateway");
                break;
            }
            Some(reconstructed_message) = mixnet_client.next() => {
                let Some(data_response) = unpack_data_response(&reconstructed_message) else {
                    continue;
                };

                // IP packets are bundled together in a mixnet message
                let mut bytes = BytesMut::from(&*data_response.ip_packet);
                while let Ok(Some(packet)) = multi_ip_packet_decoder.decode(&mut bytes) {
                    if let Some(event) = check_for_icmp_beacon_reply(&packet.into_bytes(), icmp_identifier(), our_ips) {
                        info!("Received ICMP echo reply from exit gateway");
                        info!("Connection event: {event:?}");
                        registered_replies.register_event(&event);
                    }
                }
            }
        }
    }

    Ok(Some(Exit {
        can_connect: true,
        can_route_ip_v4: registered_replies.ipr_tun_ip_v4,
        can_route_ip_external_v4: registered_replies.external_ip_v4,
        can_route_ip_v6: registered_replies.ipr_tun_ip_v6,
        can_route_ip_external_v6: registered_replies.external_ip_v6,
    }))
}

fn unpack_data_response(reconstructed_message: &ReconstructedMessage) -> Option<DataResponse> {
    match IpPacketResponse::from_reconstructed_message(reconstructed_message) {
        Ok(response) => match response.data {
            IpPacketResponseData::Data(data_response) => Some(data_response),
            IpPacketResponseData::Control(control) => match *control {
                ControlResponse::Info(info) => {
                    let msg = format!("Received info response from the mixnet: {}", info.reply);
                    match info.level {
                        InfoLevel::Info => info!("{msg}"),
                        InfoLevel::Warn => warn!("{msg}"),
                        InfoLevel::Error => error!("{msg}"),
                    }
                    None
                }
                _ => {
                    info!("Ignoring: {:?}", control);
                    None
                }
            },
        },
        Err(err) => {
            warn!("Failed to parse mixnet message: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netstack_args_default_values() {
        // Test that the default values are correctly set in the struct definition
        // This validates that our changes to the default values are correct

        // Create a default instance to test the values
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string()],
            netstack_ping_hosts_v6: vec!["cloudflare.com".to_string()],
            netstack_ping_ips_v6: vec![
                "2001:4860:4860::8888".to_string(),
                "2606:4700:4700::1111".to_string(),
                "2620:fe::fe".to_string(),
            ],
        };

        // Test IPv4 defaults
        assert_eq!(args.netstack_ping_hosts_v4, vec!["nym.com"]);
        assert_eq!(args.netstack_ping_ips_v4, vec!["1.1.1.1"]);
        assert_eq!(args.netstack_v4_dns, "1.1.1.1");

        // Test IPv6 defaults
        assert_eq!(args.netstack_ping_hosts_v6, vec!["cloudflare.com"]);
        assert_eq!(
            args.netstack_ping_ips_v6,
            vec![
                "2001:4860:4860::8888",
                "2606:4700:4700::1111",
                "2620:fe::fe"
            ]
        );
        assert_eq!(args.netstack_v6_dns, "2606:4700:4700::1111");

        // Test other defaults
        assert_eq!(args.netstack_download_timeout_sec, 180);
        assert_eq!(args.netstack_num_ping, 5);
        assert_eq!(args.netstack_send_timeout_sec, 3);
        assert_eq!(args.netstack_recv_timeout_sec, 3);
    }

    #[test]
    fn test_netstack_args_custom_construction() {
        // Test that we can create instances with custom values
        let args = NetstackArgs {
            netstack_download_timeout_sec: 300,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "8.8.8.8".to_string(),
            netstack_v6_dns: "2001:4860:4860::8888".to_string(),
            netstack_num_ping: 10,
            netstack_send_timeout_sec: 5,
            netstack_recv_timeout_sec: 5,
            netstack_ping_hosts_v4: vec!["example.com".to_string()],
            netstack_ping_ips_v4: vec!["8.8.8.8".to_string()],
            netstack_ping_hosts_v6: vec!["ipv6.example.com".to_string()],
            netstack_ping_ips_v6: vec!["2001:4860:4860::8888".to_string()],
        };

        assert_eq!(args.netstack_ping_hosts_v4, vec!["example.com"]);
        assert_eq!(args.netstack_ping_hosts_v6, vec!["ipv6.example.com"]);
        assert_eq!(args.netstack_ping_ips_v4, vec!["8.8.8.8"]);
        assert_eq!(args.netstack_ping_ips_v6, vec!["2001:4860:4860::8888"]);
        assert_eq!(args.netstack_v4_dns, "8.8.8.8");
        assert_eq!(args.netstack_v6_dns, "2001:4860:4860::8888");
        assert_eq!(args.netstack_download_timeout_sec, 300);
        assert_eq!(args.netstack_num_ping, 10);
        assert_eq!(args.netstack_send_timeout_sec, 5);
        assert_eq!(args.netstack_recv_timeout_sec, 5);
    }

    #[test]
    fn test_netstack_args_multiple_values() {
        // Test that multiple hosts and IPs can be stored
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string(), "example.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            netstack_ping_hosts_v6: vec![
                "cloudflare.com".to_string(),
                "ipv6.example.com".to_string(),
            ],
            netstack_ping_ips_v6: vec![
                "2001:4860:4860::8888".to_string(),
                "2606:4700:4700::1111".to_string(),
            ],
        };

        assert_eq!(args.netstack_ping_hosts_v4, vec!["nym.com", "example.com"]);
        assert_eq!(
            args.netstack_ping_hosts_v6,
            vec!["cloudflare.com", "ipv6.example.com"]
        );
        assert_eq!(args.netstack_ping_ips_v4, vec!["1.1.1.1", "8.8.8.8"]);
        assert_eq!(
            args.netstack_ping_ips_v6,
            vec!["2001:4860:4860::8888", "2606:4700:4700::1111"]
        );
    }

    #[test]
    fn test_netstack_args_edge_cases() {
        // Test edge cases like zero values and empty vectors
        let args = NetstackArgs {
            netstack_download_timeout_sec: 0,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 0,
            netstack_send_timeout_sec: 0,
            netstack_recv_timeout_sec: 0,
            netstack_ping_hosts_v4: vec![],
            netstack_ping_ips_v4: vec![],
            netstack_ping_hosts_v6: vec![],
            netstack_ping_ips_v6: vec![],
        };

        assert_eq!(args.netstack_num_ping, 0);
        assert_eq!(args.netstack_send_timeout_sec, 0);
        assert_eq!(args.netstack_recv_timeout_sec, 0);
        assert_eq!(args.netstack_download_timeout_sec, 0);
        assert!(args.netstack_ping_hosts_v4.is_empty());
        assert!(args.netstack_ping_ips_v4.is_empty());
        assert!(args.netstack_ping_hosts_v6.is_empty());
        assert!(args.netstack_ping_ips_v6.is_empty());
    }

    #[test]
    fn test_netstack_args_domain_validation() {
        // Test that our domain choices are reasonable
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string()],
            netstack_ping_hosts_v6: vec!["cloudflare.com".to_string()],
            netstack_ping_ips_v6: vec!["2001:4860:4860::8888".to_string()],
        };

        assert!(args.netstack_ping_hosts_v4[0].contains("nym"));

        assert!(args.netstack_ping_hosts_v6[0].contains("cloudflare"));

        assert_eq!(args.netstack_v4_dns, "1.1.1.1");
        assert_eq!(args.netstack_v6_dns, "2606:4700:4700::1111");
    }
}
