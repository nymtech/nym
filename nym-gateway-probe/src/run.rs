// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_config::defaults::setup_env;
use nym_gateway_probe::nodes::NymApiDirectory;
use nym_gateway_probe::{CredentialArgs, NetstackArgs, ProbeResult, TestedNode};
use nym_sdk::NymNetworkDetails;
use nym_sdk::mixnet::NodeIdentity;
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
        /// Provide a mnemonic to get credentials
        #[arg(long)]
        mnemonic: String,

        #[arg(long)]
        config_dir: Option<PathBuf>,
    },
    Socks5 {
        /// if not provided, test a random gateway
        #[arg(long)]
        gateway_key: Option<String>,
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
    let api_url = network
        .endpoints
        .first()
        .and_then(|ep| ep.api_url())
        .ok_or(anyhow::anyhow!("missing nyxd url"))?;

    let directory = NymApiDirectory::new(api_url).await?;

    let node_override = args.node;
    let entry_override = if let Some(gateway) = &args.entry_gateway {
        Some(NodeIdentity::from_base58_string(gateway)?)
    } else {
        None
    };

    let entry = if let Some(entry) = entry_override {
        entry
    } else if let Some(node) = node_override {
        node
    } else {
        directory.random_entry_gateway()?
    };

    let test_point = match (node_override, entry_override) {
        (Some(node), Some(_)) => TestedNode::Custom {
            identity: node,
            shares_entry: false,
        },
        (Some(node), None) => TestedNode::Custom {
            identity: node,
            shares_entry: true,
        },
        (None, _) => TestedNode::SameAsEntry,
    };

    let mut trial =
        nym_gateway_probe::Probe::new(entry, test_point, args.netstack_args, args.credential_args);
    if let Some(awg_args) = args.amnezia_args {
        trial.with_amnezia(&awg_args);
    }

    match args.command {
        Some(Commands::RunLocal {
            mnemonic,
            config_dir,
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
                &mnemonic,
                directory,
                nyxd_url,
                args.ignore_egress_epoch_role,
                args.only_wireguard,
                args.min_gateway_mixnet_performance,
            ))
            .await
        }
        Some(Commands::Socks5 { gateway_key }) => {
            let network_details = NymNetworkDetails::new_from_env();
            Box::pin(trial.test_socks5_only(directory, gateway_key, network_details)).await
        }
        None => {
            Box::pin(trial.probe(
                directory,
                nyxd_url,
                args.ignore_egress_epoch_role,
                args.only_wireguard,
                args.min_gateway_mixnet_performance,
            ))
            .await
        }
    }
}
