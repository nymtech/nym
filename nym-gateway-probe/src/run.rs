// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use nym_bin_common::bin_info;
use nym_config::defaults::setup_env;
use nym_gateway_probe::config::{CredentialArgs, CredentialMode, NetstackArgs, ProbeConfig};
use nym_gateway_probe::{
    DirectPortCheckProtocol, NymApiDirectory, PortCheckResult, ProbeResult, RunPortsConfig,
    query_gateway_by_ip,
};
use nym_sdk::mixnet::NodeIdentity;
use serde::Serialize;
use std::path::Path;
use std::{path::PathBuf, sync::OnceLock};
use tracing::*;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}
const DEFAULT_CONFIG_DIR: &str = "/tmp/nym-gateway-probe/config/";

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    /// Path pointing to an env file describing the network.
    #[arg(short, long)]
    config_env_file: Option<PathBuf>,

    /// Disable logging during probe
    #[arg(long, global = true)]
    no_log: bool,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the probe on an unannounced gateway. IP must be provided. Bypasses directory lookup
    RunLocal {
        /// Directory for credential and mixnet storage
        #[arg(long)]
        config_dir: Option<PathBuf>,

        /// The address of the gateway
        /// Supports formats: IP (192.168.66.5), IP:PORT (192.168.66.5:8080), HOST:PORT (localhost:30004)
        #[arg(long)]
        entry_gateway_ip: String,

        /// The address of the exit gateway. If not provided, entry acts as exit
        /// Supports formats: IP (192.168.66.5), IP:PORT (192.168.66.5:8080), HOST:PORT (localhost:30004)
        #[arg(long)]
        exit_gateway_ip: Option<String>,

        /// Arguments to manage credentials
        #[command(flatten)]
        credential_mode: CredentialMode,

        #[command(flatten)]
        probe_config: ProbeConfig,
    },

    /// Run the probe on a bonded gateway. Uses directory lookup
    Run {
        /// Directory for credential and mixnet storage
        #[arg(long)]
        config_dir: Option<PathBuf>,

        /// The specific gateway specified by ID.
        #[arg(long, short = 'g', alias = "gateway")]
        entry_gateway: NodeIdentity,

        /// Optional identity of the exit node to test, if not provided, entry_gateway is used
        #[arg(long)]
        exit_gateway: Option<NodeIdentity>,

        /// Arguments to manage credentials
        #[command(flatten)]
        credential_mode: CredentialMode,

        #[command(flatten)]
        probe_config: ProbeConfig,
    },

    /// Check WG exit policy ports on a bonded gateway.
    /// Tests TCP connectivity through the WG tunnel for each port.
    /// Use --check-ports to pick specific ports, or --check-all-ports for the full exit policy list.
    RunPorts {
        /// Directory for credential and mixnet storage
        #[arg(long)]
        config_dir: Option<PathBuf>,

        /// Bonded gateway identity.
        /// Cannot be used with --gateway-ip.
        #[arg(long, short = 'g', alias = "gateway", conflicts_with = "gateway_ip")]
        entry_gateway: Option<NodeIdentity>,

        /// Gateway queried directly by IP address (unannounced/local gateways)
        /// Cannot be used with --gateway/--entry-gateway.
        /// Cannot be used with --mnemonic or --use-mock-ecash.
        #[arg(long, conflicts_with = "entry_gateway", conflicts_with = "gateway")]
        gateway_ip: Option<String>,

        /// Separate exit gateway to test (entry_gateway is used for mixnet entry)
        /// Cannot be used with --gateway-ip.
        #[arg(long, conflicts_with = "gateway_ip")]
        exit_gateway: Option<NodeIdentity>,

        /// Test every port in the canonical exit policy (network-tunnel-manager.sh PORT_MAPPINGS).
        /// Overrides --check-ports.
        #[arg(long)]
        check_all_ports: bool,

        /// Specify the protocol used for tests with --gateway-ip.
        #[arg(long, value_enum, default_value_t = PortCheckProtocol::Auto)]
        check_protocol: PortCheckProtocol,

        /// Optional credential arguments.
        /// Required only in bonded mode (when using --gateway/--entry-gateway).
        #[command(flatten)]
        credential_mode: OptionalCredentialMode,

        #[command(flatten)]
        probe_config: RunPortsProbeConfig,
    },

    /// Run the probe by NS agents
    RunAgent {
        /// The specific gateway specified by ID.
        #[arg(long, short = 'g', alias = "gateway")]
        entry_gateway: NodeIdentity,

        /// Arguments to manage credentials
        #[command(flatten)]
        credential_args: CredentialArgs,

        #[command(flatten)]
        probe_config: ProbeConfig,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum PortCheckProtocol {
    Auto,
    Tcp,
    Udp,
}

#[derive(Debug, Args, Clone)]
#[command(group(
    ArgGroup::new("run_ports_credential_mode")
        .args(["use_mock_ecash","mnemonic"])
        .required(false)
        .multiple(false)
))]
struct OptionalCredentialMode {
    /// Use mock ecash credentials for testing
    #[arg(long, action = clap::ArgAction::SetTrue, conflicts_with = "gateway_ip")]
    use_mock_ecash: bool,

    /// Mnemonic to get credentials from the blockchain
    #[arg(long, conflicts_with = "gateway_ip")]
    mnemonic: Option<String>,
}

impl OptionalCredentialMode {
    fn into_required(self) -> anyhow::Result<CredentialMode> {
        if self.use_mock_ecash || self.mnemonic.is_some() {
            Ok(CredentialMode {
                use_mock_ecash: self.use_mock_ecash,
                mnemonic: self.mnemonic,
            })
        } else {
            anyhow::bail!(
                "missing credentials for bonded run-ports mode: provide --mnemonic <MNEMONIC> or --use-mock-ecash"
            )
        }
    }
}

#[derive(Debug, Args, Clone)]
struct RunPortsProbeConfig {
    /// Only choose gateway with that minimum performance
    #[arg(long)]
    min_gateway_mixnet_performance: Option<u8>,

    /// Ignore egress epoch role constraints
    #[arg(long, global = true)]
    ignore_egress_epoch_role: bool,

    /// Arguments to manage netstack downloads and port checks
    #[command(flatten)]
    netstack_args: NetstackArgs,
}

/// CLI output wrapper — either a standard probe result or a port-check result
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum ProbeOutput {
    Standard(ProbeResult),
    PortCheck(PortCheckResult),
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

pub(crate) async fn run() -> anyhow::Result<ProbeOutput> {
    let args = CliArgs::parse();
    if !args.no_log {
        setup_logging();
    }
    debug!("{:?}", nym_bin_common::bin_info_local_vergen!());

    setup_env(args.config_env_file.as_ref());
    let network = nym_sdk::NymNetworkDetails::new_from_env();

    info!("{:#?}", network);

    match args.command {
        Commands::RunLocal {
            config_dir,
            entry_gateway_ip,
            exit_gateway_ip,
            credential_mode,
            probe_config,
        } => {
            info!("Using direct IP query mode for gateway: {entry_gateway_ip}");
            let entry_details = query_gateway_by_ip(entry_gateway_ip)
                .await?
                .to_testable_node()?;

            // Parse exit gateway if provided
            let exit_details = if let Some(ip_address) = exit_gateway_ip {
                info!("Using direct IP query mode for exit gateway: {ip_address}");
                Some(query_gateway_by_ip(ip_address).await?.to_testable_node()?)
            } else {
                None
            };

            let config_dir = config_dir
                .clone()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR).join(&network.network_name));

            if config_dir.is_file() {
                anyhow::bail!("provided configuration directory is a file");
            }

            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir.clone())?;
            }

            info!(
                "using the following directory for the probe config: {}",
                config_dir.display()
            );

            let trial =
                nym_gateway_probe::Probe::new(entry_details, exit_details, network, probe_config);

            Box::pin(trial.probe_run_locally(&config_dir, credential_mode))
                .await
                .map(ProbeOutput::Standard)
        }
        Commands::Run {
            entry_gateway,
            exit_gateway,
            config_dir,
            credential_mode,
            probe_config,
        } => {
            let api_url = network
                .endpoints
                .first()
                .and_then(|ep| ep.api_url())
                .ok_or(anyhow::anyhow!("missing api url"))?;

            let directory = NymApiDirectory::new(api_url).await?;
            let entry_details = directory
                .entry_gateway(&entry_gateway)?
                .to_testable_node()?;
            let exit_details = exit_gateway
                .map(|id_key| directory.exit_gateway(&id_key))
                .transpose()?
                .map(|node| node.to_testable_node())
                .transpose()?;

            let config_dir = config_dir
                .clone()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR).join(&network.network_name));

            if config_dir.is_file() {
                anyhow::bail!("provided configuration directory is a file");
            }

            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir.clone())?;
            }

            info!(
                "using the following directory for the probe config: {}",
                config_dir.display()
            );

            let trial =
                nym_gateway_probe::Probe::new(entry_details, exit_details, network, probe_config);
            Box::pin(trial.probe_run(&config_dir, credential_mode))
                .await
                .map(ProbeOutput::Standard)
        }
        Commands::RunPorts {
            entry_gateway,
            gateway_ip,
            exit_gateway,
            config_dir,
            check_all_ports,
            check_protocol,
            credential_mode,
            probe_config,
        } => {
            let mut run_ports_config = RunPortsConfig {
                min_gateway_mixnet_performance: probe_config.min_gateway_mixnet_performance,
                ignore_egress_epoch_role: probe_config.ignore_egress_epoch_role,
                netstack_args: probe_config.netstack_args,
            };

            // --check-all-ports overrides --check-ports with the full exit policy list
            if check_all_ports {
                use nym_gateway_probe::config::EXIT_POLICY_PORTS;
                run_ports_config.netstack_args.port_check_ports = EXIT_POLICY_PORTS.to_vec();
                info!(
                    "Using full exit policy port list ({} ports)",
                    EXIT_POLICY_PORTS.len()
                );
            }

            if let Some(gateway_ip) = gateway_ip {
                info!("Using direct IP-only port check mode for gateway: {gateway_ip}");
                if entry_gateway.is_some() {
                    anyhow::bail!("--gateway/--entry-gateway cannot be used with --gateway-ip");
                }

                let target_ip: std::net::IpAddr = gateway_ip.parse().map_err(|_| {
                    anyhow::anyhow!(
                        "invalid --gateway-ip value '{gateway_ip}': expected plain IP address"
                    )
                })?;

                let direct_protocol = match check_protocol {
                    PortCheckProtocol::Auto => DirectPortCheckProtocol::Auto,
                    PortCheckProtocol::Tcp => DirectPortCheckProtocol::Tcp,
                    PortCheckProtocol::Udp => DirectPortCheckProtocol::Udp,
                };

                return Box::pin(nym_gateway_probe::Probe::probe_run_ports_direct_ip(
                    &gateway_ip,
                    target_ip,
                    &run_ports_config,
                    direct_protocol,
                ))
                .await
                .map(ProbeOutput::PortCheck);
            }

            if check_protocol == PortCheckProtocol::Udp {
                anyhow::bail!(
                    "--check-protocol udp is only supported with --gateway-ip direct mode"
                );
            }
            if check_protocol == PortCheckProtocol::Auto {
                info!(
                    "Bonded run-ports mode uses TCP checks; treating --check-protocol auto as tcp"
                );
            }

            let credential_mode = credential_mode.into_required()?;

            let api_url = network
                .endpoints
                .first()
                .and_then(|ep| ep.api_url())
                .ok_or(anyhow::anyhow!("missing api url"))?;

            let directory = NymApiDirectory::new(api_url).await?;
            let entry_gateway = entry_gateway.ok_or_else(|| {
                anyhow::anyhow!(
                    "missing gateway selection: provide --gateway <ID> or --gateway-ip <IP[:PORT]>"
                )
            })?;

            let entry_details = directory
                .entry_gateway(&entry_gateway)?
                .to_testable_node()?;

            let exit_details = exit_gateway
                .map(|id_key| directory.exit_gateway(&id_key))
                .transpose()?
                .map(|node| node.to_testable_node())
                .transpose()?;

            let config_dir = config_dir
                .clone()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR).join(&network.network_name));

            if config_dir.is_file() {
                anyhow::bail!("provided configuration directory is a file");
            }

            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir.clone())?;
            }

            info!(
                "using the following directory for the probe config: {}",
                config_dir.display()
            );

            Box::pin(nym_gateway_probe::Probe::run_ports_bonded(
                entry_details,
                exit_details,
                network,
                &run_ports_config,
                &config_dir,
                credential_mode,
            ))
            .await
            .map(ProbeOutput::PortCheck)
        }
        Commands::RunAgent {
            entry_gateway,
            credential_args,
            probe_config,
        } => {
            let trial =
                nym_gateway_probe::Probe::new_for_agent(entry_gateway, network, probe_config)
                    .await?;
            Box::pin(trial.probe_run_agent(credential_args)).await
        }
    }
}
