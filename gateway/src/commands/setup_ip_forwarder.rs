// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::helpers::{
    initialise_local_ip_forwarder, try_load_current_config, OverrideIpForwarderConfig,
};
use crate::node::helpers::load_public_key;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct CmdArgs {
    /// The id of the gateway you want to initialise local ip forwarder for.
    #[arg(long)]
    id: String,

    /// Path to custom location for ip forward's config.
    #[arg(long)]
    custom_config_path: Option<PathBuf>,

    /// Specify whether the ip forwarder should be enabled.
    // (you might want to create all the configs, generate keys, etc. but not actually run the NR just yet)
    #[arg(long)]
    enabled: Option<bool>,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<&CmdArgs> for OverrideIpForwarderConfig {
    fn from(_value: &CmdArgs) -> Self {
        OverrideIpForwarderConfig {}
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
        config.storage_paths.ip_forwarder_config = Some(custom_config_path);
    }

    if let Some(override_enabled) = args.enabled {
        config.ip_forwarder.enabled = override_enabled;
    }

    if config.storage_paths.ip_forwarder_config.is_none() {
        config = config.with_default_ip_forwarder_config_path()
    }

    let identity_public_key = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;
    let details = initialise_local_ip_forwarder(&config, opts, identity_public_key).await?;
    config.try_save()?;

    args.output.to_stdout(&details);

    Ok(())
}
