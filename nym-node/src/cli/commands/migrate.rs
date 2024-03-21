// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::{
    EntryGatewayArgs, ExitGatewayArgs, HostArgs, HttpArgs, MixnetArgs, MixnodeArgs, WireguardArgs,
};
use crate::node::helpers::load_ed25519_identity_public_key;
use clap::ValueEnum;
use nym_gateway::GatewayError;
use nym_mixnode::MixnodeError;
use nym_node::config;
use nym_node::config::Config;
use nym_node::config::{default_config_filepath, ConfigBuilder, NodeMode};
use nym_node::error::{EntryGatewayError, NymNodeError};
use std::fmt::{Display, Formatter};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::{info, trace, warn};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NodeType {
    Mixnode,
    Gateway,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Mixnode => write!(f, "mixnode"),
            NodeType::Gateway => write!(f, "gateway"),
        }
    }
}

#[derive(clap::Args, Debug)]
#[clap(group = clap::ArgGroup::new("old-config").required(true))]
pub(crate) struct Args {
    /// Type of node (mixnode or gateway) to migrate into a nym-node.
    #[clap(long)]
    node_type: NodeType,

    /// Id of the node that's going to get migrated
    #[clap(long, group = "old-config")]
    id: Option<String>,

    /// Path to a configuration file of the node that's going to get migrated.
    #[clap(long, group = "old-config")]
    config_file: Option<PathBuf>,

    /// Specify whether to preserve id of the imported node.
    #[clap(long)]
    preserve_id: bool,

    // totally optional arguments for overriding any defaults:
    #[clap(flatten)]
    host: HostArgs,

    #[clap(flatten)]
    http: HttpArgs,

    #[clap(flatten)]
    mixnet: MixnetArgs,

    #[clap(flatten)]
    wireguard: WireguardArgs,

    #[clap(flatten)]
    mixnode: MixnodeArgs,

    #[clap(flatten)]
    entry_gateway: EntryGatewayArgs,

    #[clap(flatten)]
    exit_gateway: ExitGatewayArgs,
}

impl Args {
    fn take_mnemonic(&mut self) -> Option<Zeroizing<bip39::Mnemonic>> {
        self.entry_gateway.mnemonic.take().map(Zeroizing::new)
    }

    fn config_path(&self) -> PathBuf {
        // SAFETY:
        // if `config_file` hasn't been specified, `id` MUST be available due to clap's ArgGroup
        #[allow(clippy::unwrap_used)]
        self.config_file.clone().unwrap_or({
            let id = self.id.as_ref().unwrap();
            match self.node_type {
                NodeType::Mixnode => nym_mixnode::config::default_config_filepath(id),
                NodeType::Gateway => nym_gateway::config::default_config_filepath(id),
            }
        })
    }
}

fn nym_node_id(
    typ: NodeType,
    original_id: &str,
    preserve_id: bool,
) -> Result<String, NymNodeError> {
    if preserve_id {
        let path = default_config_filepath(original_id);
        if path.exists() {
            return Err(NymNodeError::MigrationFailure {
                node_type: typ.to_string(),
                message: format!("nym-node with id '{original_id}' already exists"),
            });
        }
    }

    let mut candidate = original_id.to_string();
    let mut counter = 0;
    loop {
        let path = default_config_filepath(&candidate);
        if path.exists() {
            warn!("nym-node with id '{candidate}' already exists")
        } else {
            return Ok(candidate);
        }

        candidate = format!("{original_id}-{counter}");
        counter += 1;
    }
}

fn copy_old_data<P: AsRef<Path>, Q: AsRef<Path>>(
    node_type: NodeType,
    from: P,
    to: Q,
) -> Result<(), NymNodeError> {
    if let Err(err) = fs::copy(from.as_ref(), to.as_ref()) {
        return Err(NymNodeError::MigrationFailure {
            node_type: node_type.to_string(),
            message: format!(
                "failed to move '{}' to '{}': {err}",
                from.as_ref().display(),
                to.as_ref().display()
            ),
        });
    }
    Ok(())
}

async fn migrate_mixnode(mut args: Args) -> Result<(), NymNodeError> {
    let maybe_custom_mnemonic = args.take_mnemonic();
    let config_file = args.config_path();
    let preserve_id = args.preserve_id;

    info!(
        "attempting to migrate mixnode from '{}'",
        config_file.display()
    );
    let cfg = nym_mixnode::config::Config::read_from_toml_file(&config_file).map_err(|source| {
        MixnodeError::ConfigLoadFailure {
            id: "???".to_string(),
            path: config_file,
            source,
        }
    })?;

    let nymnode_id = nym_node_id(NodeType::Mixnode, &cfg.mixnode.id, preserve_id)?;
    let nym_node_config_path = default_config_filepath(&nymnode_id);
    let data_dir = Config::default_data_directory(&nym_node_config_path)?;

    // SAFETY:
    // our default location is never the root directory
    #[allow(clippy::unwrap_used)]
    let nym_node_config_dir = nym_node_config_path.parent().unwrap().to_path_buf();

    let ip = cfg.mixnode.listening_address;

    // generate nym-node config
    let config = ConfigBuilder::new(nymnode_id, nym_node_config_path, data_dir.clone())
        .with_mode(NodeMode::Mixnode)
        .with_host(args.host.override_config_section(config::Host {
            public_ips: cfg.host.public_ips,
            hostname: cfg.host.hostname,
            ..Default::default()
        }))
        .with_http(args.http.override_config_section(config::Http {
            bind_address: cfg.http.bind_address,
            landing_page_assets_path: cfg.http.landing_page_assets_path,
            access_token: cfg.http.metrics_key,
        }))
        .with_mixnet(args.mixnet.override_config_section(config::Mixnet {
            bind_address: SocketAddr::new(ip, cfg.mixnode.mix_port),
            nym_api_urls: cfg.mixnode.nym_api_urls,
            debug: config::MixnetDebug {
                packet_forwarding_initial_backoff: cfg.debug.packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: cfg.debug.packet_forwarding_maximum_backoff,
                initial_connection_timeout: cfg.debug.initial_connection_timeout,
                maximum_connection_buffer_size: cfg.debug.maximum_connection_buffer_size,
            },
            ..Default::default()
        }))
        .with_mixnode(args.mixnode.override_config_section(config::MixnodeConfig {
            verloc: config::mixnode::Verloc {
                bind_address: SocketAddr::new(ip, cfg.mixnode.verloc_port),
                debug: config::mixnode::VerlocDebug {
                    packets_per_node: cfg.verloc.packets_per_node,
                    connection_timeout: cfg.verloc.connection_timeout,
                    packet_timeout: cfg.verloc.packet_timeout,
                    delay_between_packets: cfg.verloc.delay_between_packets,
                    tested_nodes_batch_size: cfg.verloc.tested_nodes_batch_size,
                    testing_interval: cfg.verloc.testing_interval,
                    retry_timeout: cfg.verloc.retry_timeout,
                },
            },
            debug: config::mixnode::Debug {
                node_stats_logging_delay: cfg.debug.node_stats_logging_delay,
                node_stats_updating_delay: cfg.debug.node_stats_updating_delay,
            },
            ..config::MixnodeConfig::new_default(nym_node_config_dir)
        }))
        .with_wireguard(args.wireguard.build_config_section(&data_dir))
        .with_entry_gateway(args.entry_gateway.build_config_section(&data_dir))
        .with_exit_gateway(args.exit_gateway.build_config_section(&data_dir))
        .build()?;

    // move existing keys and generate missing data
    info!("attempting to copy mixnode keys to their new locations");
    copy_old_data(
        NodeType::Mixnode,
        cfg.storage_paths.node_description,
        &config.mixnode.storage_paths.node_description,
    )?;
    copy_old_data(
        NodeType::Mixnode,
        cfg.storage_paths.keys.public_identity_key_file,
        &config.storage_paths.keys.public_ed25519_identity_key_file,
    )?;
    copy_old_data(
        NodeType::Mixnode,
        cfg.storage_paths.keys.private_identity_key_file,
        &config.storage_paths.keys.private_ed25519_identity_key_file,
    )?;
    copy_old_data(
        NodeType::Mixnode,
        cfg.storage_paths.keys.public_sphinx_key_file,
        &config.storage_paths.keys.public_x25519_sphinx_key_file,
    )?;
    copy_old_data(
        NodeType::Mixnode,
        cfg.storage_paths.keys.private_sphinx_key_file,
        &config.storage_paths.keys.private_x25519_sphinx_key_file,
    )?;

    let ed25519_public_key = load_ed25519_identity_public_key(
        &config.storage_paths.keys.public_ed25519_identity_key_file,
    )?;

    // entry gateway initialisation
    crate::node::EntryGatewayData::initialise(&config.entry_gateway, maybe_custom_mnemonic)?;

    // exit gateway initialisation
    crate::node::ExitGatewayData::initialise(&config.exit_gateway, ed25519_public_key).await?;

    info!(
        "mixnode {} has been migrated into a nym-node! all of it's data can now be deleted",
        cfg.mixnode.id
    );

    Ok(())
}

fn migrate_gateway(args: Args) -> Result<(), NymNodeError> {
    let config_file = args.config_path();
    let preserve_id = args.preserve_id;

    info!(
        "attempting to migrate gateway from '{}'",
        config_file.display()
    );
    let cfg = nym_gateway::config::Config::read_from_toml_file(&config_file)
        .map_err(|source| GatewayError::ConfigLoadFailure {
            id: "???".to_string(),
            path: config_file,
            source,
        })
        .map_err(EntryGatewayError::from)?;

    let nymnode_id = nym_node_id(NodeType::Gateway, &cfg.gateway.id, preserve_id)?;
    let nym_node_config_path = default_config_filepath(&nymnode_id);
    let data_dir = Config::default_data_directory(&nym_node_config_path)?;

    // TODO: exit vs entry will be determined based on if IPR **AND** NR are enabled
    let config = ConfigBuilder::new(nymnode_id, nym_node_config_path, data_dir);
    let _ = config;

    todo!()
}

pub(crate) async fn execute(args: Args) -> Result<(), NymNodeError> {
    trace!("args: {args:#?}");

    match args.node_type {
        NodeType::Mixnode => migrate_mixnode(args).await,
        NodeType::Gateway => migrate_gateway(args),
    }
}
