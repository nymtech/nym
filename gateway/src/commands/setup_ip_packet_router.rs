// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::{initialise_local_ip_packet_router, try_load_current_config};
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_gateway::helpers::{load_public_key, OverrideIpPacketRouterConfig};
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct CmdArgs {
    /// The id of the gateway you want to initialise local ip packet router for.
    #[arg(long)]
    id: String,

    /// Path to custom location for ip packet routers' config.
    #[arg(long)]
    custom_config_path: Option<PathBuf>,

    /// Specify whether the ip packet router should be enabled.
    // (you might want to create all the configs, generate keys, etc. but not actually run the NR just yet)
    #[arg(long)]
    enabled: Option<bool>,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<&CmdArgs> for OverrideIpPacketRouterConfig {
    fn from(_value: &CmdArgs) -> Self {
        OverrideIpPacketRouterConfig {}
    }
}

pub async fn execute(args: CmdArgs) -> anyhow::Result<()> {
    let mut config = try_load_current_config(&args.id)?;
    let opts = (&args).into();

    // if somebody provided config file of a custom NR, that's fine
    // but in 90% cases, I'd assume, it won't work due to invalid gateway configuration
    // but it might be nice to be able to move files around.
    if let Some(custom_config_path) = args.custom_config_path {
        // if you specified anything as the argument, overwrite whatever was already in the config file
        config.storage_paths.ip_packet_router_config = Some(custom_config_path);
    }

    if let Some(override_enabled) = args.enabled {
        config.ip_packet_router.enabled = override_enabled;
    }

    if config.storage_paths.ip_packet_router_config.is_none() {
        config = config.with_default_ip_packet_router_config_path()
    }

    let identity_public_key = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;
    let details = initialise_local_ip_packet_router(&config, opts, identity_public_key).await?;
    config.try_save()?;

    args.output.to_stdout(&details);

    Ok(())
}
