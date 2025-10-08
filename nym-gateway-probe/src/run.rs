// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_config::defaults::setup_env;
use nym_gateway_probe::nodes::NymApiDirectory;
use nym_gateway_probe::{CredentialArgs, NetstackArgs, ProbeResult, TestedNode};
use nym_sdk::mixnet::NodeIdentity;
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

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the probe locally
    RunLocal {
        /// Provide a mnemonic to get credentials
        #[arg(long)]
        mnemonic: String,

        #[arg(long, default_value = "/tmp/nym-gateway-probe/config/")]
        config_dir: PathBuf,
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

    let entry = if let Some(gateway) = &args.entry_gateway {
        NodeIdentity::from_base58_string(gateway)?
    } else {
        directory.random_exit_with_ipr()?
    };

    let test_point = if let Some(node) = args.node {
        TestedNode::Custom { identity: node }
    } else {
        TestedNode::SameAsEntry
    };

    let mut trial =
        nym_gateway_probe::Probe::new(entry, test_point, args.netstack_args, args.credential_args);
    if let Some(awg_args) = args.amnezia_args {
        trial.with_amnezia(&awg_args);
    }

    match &args.command {
        Some(Commands::RunLocal {
            mnemonic,
            config_dir,
        }) => {
            Box::pin(trial.probe_run_locally(
                config_dir,
                mnemonic,
                directory,
                nyxd_url,
                args.ignore_egress_epoch_role,
                args.only_wireguard,
                args.min_gateway_mixnet_performance,
            ))
            .await
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
