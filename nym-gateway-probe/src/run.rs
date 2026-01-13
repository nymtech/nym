// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_config::defaults::setup_env;
use nym_gateway_probe::nodes::{NymApiDirectory, query_gateway_by_ip};
use nym_gateway_probe::{
    CredentialArgs, NetstackArgs, ProbeResult, TestMode, TestedNode, TestedNodeDetails,
};
use nym_sdk::mixnet::NodeIdentity;
use std::net::SocketAddr;
use std::path::Path;
use std::{path::PathBuf, sync::OnceLock};
use tracing::*;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

fn validate_node_identity(s: &str) -> Result<NodeIdentity, String> {
    match s.parse() {
        Ok(cg) => Ok(cg),
        Err(_) => Err(format!("failed to parse country group: {s}")),
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path pointing to an env file describing the network.
    #[arg(short, long, global = true)]
    config_env_file: Option<PathBuf>,

    /// The specific gateway specified by ID.
    #[arg(long, short = 'g', alias = "gateway", global = true)]
    entry_gateway: Option<String>,

    /// The address of the gateway to probe directly (bypasses directory lookup)
    /// Supports formats: IP (192.168.66.5), IP:PORT (192.168.66.5:8080), HOST:PORT (localhost:30004)
    #[arg(long, global = true)]
    gateway_ip: Option<String>,

    /// The address of the exit gateway for LP forwarding tests (used with --test-lp-wg)
    /// When specified, --gateway-ip becomes the entry gateway and this becomes the exit gateway
    /// Supports formats: IP (192.168.66.5), IP:PORT (192.168.66.5:8080), HOST:PORT (localhost:30004)
    #[arg(long, global = true)]
    exit_gateway_ip: Option<String>,

    /// Ed25519 identity of the entry gateway (base58 encoded)
    /// When provided, skips HTTP API query - use for localnet testing
    #[arg(long, global = true)]
    entry_gateway_identity: Option<String>,

    /// Ed25519 identity of the exit gateway (base58 encoded)
    /// When provided, skips HTTP API query - use for localnet testing
    #[arg(long, global = true)]
    exit_gateway_identity: Option<String>,

    /// LP listener address for entry gateway (e.g., "192.168.66.6:41264")
    /// Used with --entry-gateway-identity for localnet mode
    #[arg(long, global = true)]
    entry_lp_address: Option<String>,

    /// LP listener address for exit gateway (e.g., "172.18.0.5:41264")
    /// This is the address the entry gateway uses to reach exit (for forwarding)
    /// Used with --exit-gateway-identity for localnet mode
    #[arg(long, global = true)]
    exit_lp_address: Option<String>,

    /// Default LP control port when deriving LP address from gateway IP
    #[arg(long, global = true, default_value = "41264")]
    lp_port: u16,

    /// Identity of the node to test
    #[arg(long, short, value_parser = validate_node_identity, global = true)]
    node: Option<NodeIdentity>,

    #[arg(long, global = true)]
    min_gateway_mixnet_performance: Option<u8>,

    // this was a dead field
    // #[arg(long, global = true)]
    // min_gateway_vpn_performance: Option<u8>,
    #[arg(long, global = true)]
    only_wireguard: bool,

    #[arg(long, global = true)]
    only_lp_registration: bool,

    /// Test WireGuard via LP registration (no mixnet) - uses nested session forwarding
    #[arg(long, global = true)]
    test_lp_wg: bool,

    /// Test mode - explicitly specify which tests to run
    ///
    /// Modes:
    ///   mixnet      - Traditional mixnet testing (entry/exit pings + WireGuard via authenticator)
    ///   single-hop  - LP registration + WireGuard on single gateway (no mixnet)
    ///   two-hop     - Entry LP + Exit LP (nested forwarding) + WireGuard tunnel
    ///   lp-only     - LP registration only (no WireGuard)
    ///
    /// If not specified, mode is inferred from other flags:
    ///   --only-lp-registration → lp-only
    ///   --test-lp-wg with exit gateway → two-hop
    ///   --test-lp-wg without exit → single-hop
    ///   otherwise → mixnet
    #[arg(long, global = true, value_name = "MODE")]
    mode: Option<String>,

    /// Disable logging during probe
    #[arg(long, global = true)]
    ignore_egress_epoch_role: bool,

    #[arg(long, global = true)]
    no_log: bool,

    /// Arguments to be appended to the wireguard config enabling amnezia-wg configuration
    #[arg(long, short, global = true)]
    amnezia_args: Option<String>,

    /// Arguments to manage netstack downloads
    #[command(flatten)]
    netstack_args: NetstackArgs,

    /// Arguments to manage credentials
    #[command(flatten)]
    credential_args: CredentialArgs,
}

const DEFAULT_CONFIG_DIR: &str = "/tmp/nym-gateway-probe/config/";

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the probe locally
    RunLocal {
        /// Provide a mnemonic to get credentials (optional when using --use-mock-ecash)
        #[arg(long)]
        mnemonic: Option<String>,

        #[arg(long)]
        config_dir: Option<PathBuf>,

        /// Use mock ecash credentials for testing (requires gateway with --lp-use-mock-ecash)
        #[arg(long)]
        use_mock_ecash: bool,
    },
}

fn setup_logging() {
    // SAFETY: those are valid directives
    #[allow(clippy::unwrap_used)]
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("hyper::proto=info".parse().unwrap())
        .add_directive("netlink_proto=info".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

/// Resolve the test mode from explicit --mode arg or infer from legacy flags
fn resolve_test_mode(
    mode_arg: Option<&str>,
    only_wireguard: bool,
    only_lp_registration: bool,
    test_lp_wg: bool,
    has_exit_gateway: bool,
) -> anyhow::Result<TestMode> {
    if let Some(mode_str) = mode_arg {
        // Explicit --mode takes priority
        mode_str
            .parse::<TestMode>()
            .map_err(|e| anyhow::anyhow!("{}", e))
    } else {
        // Infer from legacy flags
        Ok(TestMode::from_flags(
            only_wireguard,
            only_lp_registration,
            test_lp_wg,
            has_exit_gateway,
        ))
    }
}

/// Convert TestMode back to legacy boolean flags for backward compatibility
fn mode_to_flags(mode: TestMode) -> (bool, bool, bool) {
    match mode {
        TestMode::Mixnet => (false, false, false), // only_wireguard handled separately
        TestMode::SingleHop => (false, false, true),
        TestMode::TwoHop => (false, false, true),
        TestMode::LpOnly => (false, true, false),
    }
}

pub(crate) async fn run() -> anyhow::Result<ProbeResult> {
    let args = CliArgs::parse();
    if !args.no_log {
        setup_logging();
    }
    debug!("{:?}", nym_bin_common::bin_info_local_vergen!());
    setup_env(args.config_env_file.as_ref());

    let network = nym_sdk::NymNetworkDetails::new_from_env();

    let nyxd_url = network
        .endpoints
        .first()
        .map(|ep| ep.nyxd_url())
        .ok_or(anyhow::anyhow!("missing nyxd url"))?;

    // Three resolution modes in priority order:
    // 1. Localnet mode: --entry-gateway-identity provided (no HTTP query)
    // 2. Direct IP mode: --gateway-ip provided (queries HTTP API)
    // 3. Directory mode: uses nym-api directory service

    // Localnet mode: identity provided via CLI, skip HTTP queries entirely
    if let Some(entry_identity_str) = &args.entry_gateway_identity {
        info!("Using localnet mode with CLI-provided gateway identity");

        let entry_identity = NodeIdentity::from_base58_string(entry_identity_str)?;

        // Entry LP address: explicit or derived from gateway_ip + lp_port
        let entry_lp_addr: SocketAddr = if let Some(lp_addr) = &args.entry_lp_address {
            lp_addr
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid entry-lp-address '{}': {}", lp_addr, e))?
        } else if let Some(gw_ip) = &args.gateway_ip {
            // Derive LP address from gateway IP
            let ip: std::net::IpAddr = gw_ip
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid gateway-ip '{}': {}", gw_ip, e))?;
            SocketAddr::new(ip, args.lp_port)
        } else {
            anyhow::bail!(
                "--entry-lp-address or --gateway-ip required with --entry-gateway-identity"
            );
        };

        let entry_details = TestedNodeDetails::from_cli(entry_identity, entry_lp_addr);

        // Parse exit gateway if provided
        let exit_details = if let Some(exit_identity_str) = &args.exit_gateway_identity {
            let exit_identity = NodeIdentity::from_base58_string(exit_identity_str)?;
            let exit_lp_addr: SocketAddr = args
                .exit_lp_address
                .as_ref()
                .ok_or_else(|| {
                    anyhow::anyhow!("--exit-lp-address required with --exit-gateway-identity")
                })?
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid exit-lp-address: {}", e))?;
            Some(TestedNodeDetails::from_cli(exit_identity, exit_lp_addr))
        } else {
            None
        };

        // Resolve test mode from --mode arg or infer from flags
        let has_exit = exit_details.is_some();
        let test_mode = resolve_test_mode(
            args.mode.as_deref(),
            args.only_wireguard,
            args.only_lp_registration,
            args.test_lp_wg,
            has_exit,
        )?;

        // Validate that two-hop mode has required exit gateway
        if test_mode.needs_exit_gateway() && !has_exit {
            bail!(
                "--mode two-hop requires exit gateway \
                (use --exit-gateway-identity and --exit-lp-address)"
            );
        }

        info!("Test mode: {}", test_mode);

        // Convert back to flags for backward compatibility with existing probe methods
        // only_wireguard is preserved from args since it's orthogonal to mode
        // (it means "skip ping tests" in mixnet mode, irrelevant for LP modes)
        let (_, only_lp_registration, test_lp_wg) = mode_to_flags(test_mode);
        let only_wireguard = args.only_wireguard;

        let mut trial = nym_gateway_probe::Probe::new_localnet(
            entry_details,
            exit_details,
            args.netstack_args,
            args.credential_args,
        );

        if let Some(awg_args) = args.amnezia_args {
            trial.with_amnezia(&awg_args);
        }

        // Localnet mode doesn't need directory, but nyxd_url is still used for credentials
        return match &args.command {
            Some(Commands::RunLocal {
                mnemonic,
                config_dir,
                use_mock_ecash,
            }) => {
                let config_dir = config_dir
                    .clone()
                    .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR).join(&network.network_name));

                info!(
                    "using the following directory for the probe config: {}",
                    config_dir.display()
                );

                Box::pin(trial.probe_run_locally(
                    &config_dir,
                    mnemonic.as_deref(),
                    None, // No directory in localnet mode
                    nyxd_url,
                    args.ignore_egress_epoch_role,
                    only_wireguard,
                    only_lp_registration,
                    test_lp_wg,
                    args.min_gateway_mixnet_performance,
                    *use_mock_ecash,
                ))
                .await
            }
            None => {
                Box::pin(trial.probe(
                    None, // No directory in localnet mode
                    nyxd_url,
                    args.ignore_egress_epoch_role,
                    only_wireguard,
                    only_lp_registration,
                    test_lp_wg,
                    args.min_gateway_mixnet_performance,
                ))
                .await
            }
        };
    }

    // If gateway IP is provided, query it directly without using the directory
    let (entry, directory, gateway_node, exit_gateway_node) =
        if let Some(gateway_ip) = args.gateway_ip.clone() {
            info!("Using direct IP query mode for gateway: {}", gateway_ip);
            let gateway_node = query_gateway_by_ip(gateway_ip).await?;
            let identity = gateway_node.identity();

            // Query exit gateway if provided (for LP forwarding tests)
            let exit_node = if let Some(exit_gateway_ip) = args.exit_gateway_ip {
                info!(
                    "Using direct IP query mode for exit gateway: {}",
                    exit_gateway_ip
                );
                Some(query_gateway_by_ip(exit_gateway_ip).await?)
            } else {
                None
            };

            // Still create the directory for potential secondary lookups,
            // but only if API URL is available
            let directory =
                if let Some(api_url) = network.endpoints.first().and_then(|ep| ep.api_url()) {
                    Some(NymApiDirectory::new(api_url).await?)
                } else {
                    None
                };

            (identity, directory, Some(gateway_node), exit_node)
        } else {
            // Original behavior: use directory service
            let api_url = network
                .endpoints
                .first()
                .and_then(|ep| ep.api_url())
                .ok_or(anyhow::anyhow!("missing api url"))?;

            let directory = NymApiDirectory::new(api_url).await?;

            let entry = if let Some(gateway) = &args.entry_gateway {
                NodeIdentity::from_base58_string(gateway)?
            } else {
                directory.random_exit_with_ipr()?
            };

            (entry, Some(directory), None, None)
        };

    let test_point = if let Some(node) = args.node {
        TestedNode::Custom {
            identity: node,
            shares_entry: false,
        }
    } else {
        TestedNode::SameAsEntry
    };

    // Resolve test mode from --mode arg or infer from flags
    let has_exit = exit_gateway_node.is_some();
    let test_mode = resolve_test_mode(
        args.mode.as_deref(),
        args.only_wireguard,
        args.only_lp_registration,
        args.test_lp_wg,
        has_exit,
    )?;
    info!("Test mode: {}", test_mode);

    // Convert back to flags for backward compatibility with existing probe methods
    // only_wireguard is preserved from args since it's orthogonal to mode
    let (_, only_lp_registration, test_lp_wg) = mode_to_flags(test_mode);
    let only_wireguard = args.only_wireguard;

    let mut trial = if let (Some(entry_node), Some(exit_node)) = (&gateway_node, &exit_gateway_node)
    {
        // Both entry and exit gateways provided (for LP telescoping tests)
        info!("Using both entry and exit gateways for LP forwarding test");
        nym_gateway_probe::Probe::new_with_gateways(
            entry,
            test_point,
            args.netstack_args,
            args.credential_args,
            entry_node.clone(),
            exit_node.clone(),
        )
    } else if let Some(gw_node) = gateway_node {
        // Only entry gateway provided
        nym_gateway_probe::Probe::new_with_gateway(
            entry,
            test_point,
            args.netstack_args,
            args.credential_args,
            gw_node,
        )
    } else {
        // No direct gateways, use directory lookup
        nym_gateway_probe::Probe::new(entry, test_point, args.netstack_args, args.credential_args)
    };

    if let Some(awg_args) = args.amnezia_args {
        trial.with_amnezia(&awg_args);
    }

    match &args.command {
        Some(Commands::RunLocal {
            mnemonic,
            config_dir,
            use_mock_ecash,
        }) => {
            let config_dir = config_dir
                .clone()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR).join(&network.network_name));

            info!(
                "using the following directory for the probe config: {}",
                config_dir.display()
            );

            Box::pin(trial.probe_run_locally(
                &config_dir,
                mnemonic.as_deref(),
                directory,
                nyxd_url,
                args.ignore_egress_epoch_role,
                only_wireguard,
                only_lp_registration,
                test_lp_wg,
                args.min_gateway_mixnet_performance,
                *use_mock_ecash,
            ))
            .await
        }
        None => {
            Box::pin(trial.probe(
                directory,
                nyxd_url,
                args.ignore_egress_epoch_role,
                only_wireguard,
                only_lp_registration,
                test_lp_wg,
                args.min_gateway_mixnet_performance,
            ))
            .await
        }
    }
}
