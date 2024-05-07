// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::{
    EntryGatewayArgs, ExitGatewayArgs, HostArgs, HttpArgs, MixnetArgs, MixnodeArgs, WireguardArgs,
};
use crate::node::description::save_node_description;
use crate::node::helpers::{
    bonding_version, load_ed25519_identity_public_key, store_x25519_noise_keypair,
};
use clap::ValueEnum;
use colored::Color::TrueColor;
use colored::Colorize;
use nym_crypto::asymmetric::x25519;
use nym_gateway::helpers::{load_ip_packet_router_config, load_network_requester_config};
use nym_gateway::GatewayError;
use nym_mixnode::MixnodeError;
use nym_network_requester::{CustomGatewayDetails, GatewayDetails};
use nym_node::config;
use nym_node::config::mixnode::DEFAULT_VERLOC_PORT;
use nym_node::config::Config;
use nym_node::config::{default_config_filepath, ConfigBuilder, NodeMode};
use nym_node::error::{EntryGatewayError, ExitGatewayError, NymNodeError};
use nym_node_http_api::api::api_requests::v1::node::models::NodeDescription;
use rand::rngs::OsRng;
use std::fmt::{Display, Formatter};
use std::fs::create_dir_all;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::{fs, io};
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
        self.config_file.clone().unwrap_or_else(|| {
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
    fn copy_inner<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
        if let Some(parent) = to.as_ref().parent() {
            create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        Ok(())
    }

    if let Err(err) = copy_inner(from.as_ref(), to.as_ref()) {
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

    let old_description = if cfg.storage_paths.node_description.exists() {
        Some(
            nym_mixnode::node::node_description::NodeDescription::load_from_file(
                &cfg.storage_paths.node_description,
            )
            .map_err(|source| {
                nym_node::error::MixnodeError::DescriptionLoadFailure {
                    path: cfg.storage_paths.node_description,
                    source,
                }
            })?,
        )
    } else {
        None
    };

    let nymnode_id = nym_node_id(NodeType::Mixnode, &cfg.mixnode.id, preserve_id)?;
    let nym_node_config_path = default_config_filepath(&nymnode_id);
    let data_dir = Config::default_data_directory(&nym_node_config_path)?;

    let ip = cfg.mixnode.listening_address;

    let location = old_description
        .as_ref()
        .and_then(|d| d.location.parse().ok());

    // generate nym-node config
    let config = ConfigBuilder::new(nymnode_id, nym_node_config_path, data_dir.clone())
        .with_mode(NodeMode::Mixnode)
        .with_host(args.host.override_config_section(config::Host {
            public_ips: cfg.host.public_ips,
            hostname: cfg.host.hostname,
            location,
        }))
        .with_http(args.http.override_config_section(config::Http {
            bind_address: cfg.http.bind_address,
            landing_page_assets_path: cfg.http.landing_page_assets_path,
            access_token: cfg.http.metrics_key,
            ..Default::default()
        }))
        .with_mixnet(args.mixnet.override_config_section(config::Mixnet {
            bind_address: SocketAddr::new(ip, cfg.mixnode.mix_port),
            nym_api_urls: cfg.mixnode.nym_api_urls,
            debug: config::MixnetDebug {
                packet_forwarding_initial_backoff: cfg.debug.packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: cfg.debug.packet_forwarding_maximum_backoff,
                initial_connection_timeout: cfg.debug.initial_connection_timeout,
                maximum_connection_buffer_size: cfg.debug.maximum_connection_buffer_size,
                ..Default::default()
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
            ..config::MixnodeConfig::new_default()
        }))
        .with_wireguard(args.wireguard.build_config_section(&data_dir))
        .with_entry_gateway(args.entry_gateway.build_config_section(&data_dir))
        .with_exit_gateway(args.exit_gateway.build_config_section(&data_dir))
        .build();

    let d_ref = old_description.as_ref();

    // update description
    let node_description = NodeDescription {
        moniker: d_ref.map(|d| &d.name).cloned().unwrap_or_default(),
        website: d_ref.map(|d| &d.link).cloned().unwrap_or_default(),
        security_contact: "".to_string(),
        details: d_ref.map(|d| &d.description).cloned().unwrap_or_default(),
    };
    save_node_description(&config.storage_paths.description, &node_description)?;

    // create noise keypair
    let mut rng = OsRng;
    let x25519_noise_keys = x25519::KeyPair::new(&mut rng);
    trace!("attempting to store x25519 noise keypair");
    store_x25519_noise_keypair(
        &x25519_noise_keys,
        config.storage_paths.keys.x25519_noise_storage_paths(),
    )?;

    // move existing keys and generate missing data
    info!("attempting to copy mixnode keys to their new locations");
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

    crate::node::WireguardData::initialise(&config.wireguard)?;

    config.save()?;

    info!(
        "mixnode {} has been migrated into a nym-node! all of its data can now be deleted",
        cfg.mixnode.id
    );

    Ok(())
}

async fn migrate_gateway(mut args: Args) -> Result<(), NymNodeError> {
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

    let nr_cfg = match &cfg.storage_paths.network_requester_config {
        None => None,
        Some(nr_cfg) => Some(
            load_network_requester_config("???", nr_cfg)
                .await
                .map_err(ExitGatewayError::from)?,
        ),
    };

    let ipr_cfg = match &cfg.storage_paths.ip_packet_router_config {
        None => None,
        Some(ipr_cfg) => Some(
            load_ip_packet_router_config("???", ipr_cfg)
                .await
                .map_err(ExitGatewayError::from)?,
        ),
    };

    let nymnode_id = nym_node_id(NodeType::Gateway, &cfg.gateway.id, preserve_id)?;
    let nym_node_config_path = default_config_filepath(&nymnode_id);
    let data_dir = Config::default_data_directory(&nym_node_config_path)?;

    let mode = if cfg.network_requester.enabled && cfg.ip_packet_router.enabled {
        NodeMode::ExitGateway
    } else {
        NodeMode::EntryGateway
    };

    let ip = cfg.gateway.listening_address;

    // prefer new mnemonic explicitly passed with cli; otherwise use the one already present
    let mnemonic = args
        .take_mnemonic()
        .unwrap_or(Zeroizing::new(cfg.gateway.cosmos_mnemonic.clone()));

    let config = ConfigBuilder::new(nymnode_id, nym_node_config_path, data_dir.clone())
        .with_mode(mode)
        .with_host(args.host.override_config_section(config::Host {
            public_ips: cfg.host.public_ips,
            hostname: cfg.host.hostname,
            ..Default::default()
        }))
        .with_http(args.http.override_config_section(config::Http {
            bind_address: cfg.http.bind_address,
            landing_page_assets_path: cfg.http.landing_page_assets_path,
            ..Default::default()
        }))
        .with_mixnet(args.mixnet.override_config_section(config::Mixnet {
            bind_address: SocketAddr::new(ip, cfg.gateway.mix_port),
            nym_api_urls: cfg.gateway.nym_api_urls.clone(),
            nyxd_urls: cfg.gateway.nyxd_urls.clone(),
            debug: config::MixnetDebug {
                packet_forwarding_initial_backoff: cfg.debug.packet_forwarding_initial_backoff,
                packet_forwarding_maximum_backoff: cfg.debug.packet_forwarding_maximum_backoff,
                initial_connection_timeout: cfg.debug.initial_connection_timeout,
                maximum_connection_buffer_size: cfg.debug.maximum_connection_buffer_size,
                ..Default::default()
            },
        }))
        .with_mixnode(args.mixnode.override_config_section(config::MixnodeConfig {
            verloc: config::mixnode::Verloc {
                bind_address: SocketAddr::new(ip, DEFAULT_VERLOC_PORT),
                ..Default::default()
            },
            ..config::MixnodeConfig::new_default()
        }))
        .with_entry_gateway(args.entry_gateway.override_config_section(
            config::EntryGatewayConfig {
                storage_paths: config::persistence::EntryGatewayPaths::new(&data_dir),
                enforce_zk_nyms: cfg.gateway.only_coconut_credentials,
                offline_zk_nyms: cfg.gateway.offline_credential_verification,
                bind_address: SocketAddr::new(ip, cfg.gateway.clients_port),
                announce_ws_port: None,
                announce_wss_port: cfg.gateway.clients_wss_port,
                debug: config::entry_gateway::Debug {
                    message_retrieval_limit: cfg.debug.message_retrieval_limit,
                },
            },
        ))
        .with_exit_gateway(
            args.exit_gateway
                .override_config_section(config::ExitGatewayConfig {
                    storage_paths: config::persistence::ExitGatewayPaths::new(&data_dir),
                    open_proxy: false,
                    upstream_exit_policy_url: nr_cfg
                        .as_ref()
                        .and_then(|c| c.network_requester.upstream_exit_policy_url.clone())
                        .unwrap_or(
                            config::ExitGatewayConfig::new_default(".").upstream_exit_policy_url,
                        ),
                    network_requester: config::exit_gateway::NetworkRequester {
                        debug: config::exit_gateway::NetworkRequesterDebug {
                            enabled: cfg.network_requester.enabled,
                            disable_poisson_rate: nr_cfg
                                .as_ref()
                                .map(|c| c.network_requester.disable_poisson_rate)
                                .unwrap_or(
                                    config::exit_gateway::NetworkRequesterDebug::default()
                                        .disable_poisson_rate,
                                ),
                            client_debug: nr_cfg.as_ref().map(|c| c.base.debug).unwrap_or_default(),
                        },
                    },
                    ip_packet_router: config::exit_gateway::IpPacketRouter {
                        debug: config::exit_gateway::IpPacketRouterDebug {
                            enabled: cfg.ip_packet_router.enabled,
                            disable_poisson_rate: ipr_cfg
                                .as_ref()
                                .map(|c| c.ip_packet_router.disable_poisson_rate)
                                .unwrap_or(
                                    config::exit_gateway::IpPacketRouterDebug::default()
                                        .disable_poisson_rate,
                                ),
                            client_debug: ipr_cfg
                                .as_ref()
                                .map(|c| c.base.debug)
                                .unwrap_or_default(),
                        },
                    },
                }),
        )
        .build();

    // create noise keypair
    let mut rng = OsRng;
    let x25519_noise_keys = x25519::KeyPair::new(&mut rng);
    trace!("attempting to store x25519 noise keypair");
    store_x25519_noise_keypair(
        &x25519_noise_keys,
        config.storage_paths.keys.x25519_noise_storage_paths(),
    )?;

    // move existing keys and generate missing data
    info!("attempting to copy gateway keys to their new locations");

    copy_old_data(
        NodeType::Gateway,
        cfg.storage_paths.keys.public_identity_key_file,
        &config.storage_paths.keys.public_ed25519_identity_key_file,
    )?;
    copy_old_data(
        NodeType::Gateway,
        cfg.storage_paths.keys.private_identity_key_file,
        &config.storage_paths.keys.private_ed25519_identity_key_file,
    )?;
    copy_old_data(
        NodeType::Gateway,
        cfg.storage_paths.keys.public_sphinx_key_file,
        &config.storage_paths.keys.public_x25519_sphinx_key_file,
    )?;
    copy_old_data(
        NodeType::Gateway,
        cfg.storage_paths.keys.private_sphinx_key_file,
        &config.storage_paths.keys.private_x25519_sphinx_key_file,
    )?;

    let ed25519_public_key = load_ed25519_identity_public_key(
        &config.storage_paths.keys.public_ed25519_identity_key_file,
    )?;

    // mixnode data initialisation
    crate::node::MixnodeData::initialise(&config.mixnode)?;

    // selectively initialise exit gateway
    let gateway_details =
        GatewayDetails::Custom(CustomGatewayDetails::new(ed25519_public_key)).into();
    let mut rng = OsRng;

    if let Some(nr_cfg) = nr_cfg {
        let nr_paths = nr_cfg.storage_paths.common_paths;
        let new_nr_paths = &config.exit_gateway.storage_paths.network_requester;

        copy_old_data(
            NodeType::Gateway,
            nr_paths.keys.public_identity_key_file,
            &new_nr_paths.public_ed25519_identity_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.keys.private_identity_key_file,
            &new_nr_paths.private_ed25519_identity_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.keys.public_encryption_key_file,
            &new_nr_paths.public_x25519_diffie_hellman_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.keys.private_encryption_key_file,
            &new_nr_paths.private_x25519_diffie_hellman_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.keys.ack_key_file,
            &new_nr_paths.ack_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.gateway_registrations,
            &new_nr_paths.gateway_registrations,
        )?;
        copy_old_data(
            NodeType::Gateway,
            nr_paths.reply_surb_database,
            &new_nr_paths.reply_surb_database,
        )?;
    } else {
        crate::node::ExitGatewayData::initialise_network_requester(
            &mut rng,
            &config.exit_gateway,
            &gateway_details,
        )
        .await?;
    }

    if let Some(ipr_cfg) = ipr_cfg {
        let ipr_paths = ipr_cfg.storage_paths.common_paths;
        let new_ipr_paths = &config.exit_gateway.storage_paths.ip_packet_router;

        copy_old_data(
            NodeType::Gateway,
            ipr_paths.keys.public_identity_key_file,
            &new_ipr_paths.public_ed25519_identity_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.keys.private_identity_key_file,
            &new_ipr_paths.private_ed25519_identity_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.keys.public_encryption_key_file,
            &new_ipr_paths.public_x25519_diffie_hellman_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.keys.private_encryption_key_file,
            &new_ipr_paths.private_x25519_diffie_hellman_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.keys.ack_key_file,
            &new_ipr_paths.ack_key_file,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.gateway_registrations,
            &new_ipr_paths.gateway_registrations,
        )?;
        copy_old_data(
            NodeType::Gateway,
            ipr_paths.reply_surb_database,
            &new_ipr_paths.reply_surb_database,
        )?;
    } else {
        crate::node::ExitGatewayData::initialise_ip_packet_router_requester(
            &mut rng,
            &config.exit_gateway,
            &gateway_details,
        )
        .await?;
    }

    crate::node::WireguardData::initialise(&config.wireguard)?;

    save_node_description(
        &config.storage_paths.description,
        &NodeDescription::default(),
    )?;

    // finally move the mnemonic
    config
        .entry_gateway
        .storage_paths
        .save_mnemonic_to_file(&mnemonic)?;

    config.save()?;

    info!(
        "gateway {} has been migrated into a nym-node! all of its data can now be deleted",
        cfg.gateway.id
    );

    Ok(())
}

pub(crate) async fn execute(args: Args) -> Result<(), NymNodeError> {
    trace!("args: {args:#?}");

    match args.node_type {
        NodeType::Mixnode => migrate_mixnode(args).await?,
        NodeType::Gateway => migrate_gateway(args).await?,
    }

    let orange = TrueColor {
        r: 251,
        g: 110,
        b: 78,
    };

    println!("{}", "** Attention **".color(orange).bold());
    print!("Please consider updating the '");
    print!("{}", "version".color(orange));
    print!("' field of your ");
    print!("{}", "existing".bold().underline());
    println!(" node to:");
    println!();
    println!("{}", bonding_version().bold().color(orange));
    println!();
    print!("in the settings section of the ");
    println!("{}", "Nym Wallet".bold().color(orange));
    println!();

    Ok(())
}
