// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::{
    initialise_local_network_requester, try_load_current_config, OverrideNetworkRequesterConfig,
};
use crate::node::helpers::load_public_key;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use std::path::PathBuf;

#[derive(Args, Clone)]
pub struct CmdArgs {
    /// The id of the gateway you want to initialise local network requester for.
    #[clap(long)]
    id: String,

    /// Path to custom location for network requester's config.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,

    /// Specify whether the network requester should be enabled.
    // (you might want to create all the configs, generate keys, etc. but not actually run the NR just yet)
    #[clap(long)]
    enabled: Option<bool>,

    // note: those flags are set as bools as we want to explicitly override any settings values
    // so say `open_proxy` was set to true in the config.toml. youd have to explicitly state `open-proxy=false`
    // as an argument here to override it as opposed to not providing the value at all.
    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[clap(long)]
    open_proxy: Option<bool>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[clap(long)]
    enable_statistics: Option<bool>,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[clap(long)]
    statistics_recipient: Option<String>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[clap(long, hide = true, conflicts_with = "medium_toggle")]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[clap(long, hide = true, conflicts_with = "medium_toggle")]
    no_cover: bool,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[clap(
        long,
        hide = true,
        conflicts_with = "no_cover",
        conflicts_with = "fastmode"
    )]
    medium_toggle: bool,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl<'a> From<&'a CmdArgs> for OverrideNetworkRequesterConfig {
    fn from(value: &'a CmdArgs) -> Self {
        OverrideNetworkRequesterConfig {
            fastmode: value.fastmode,
            no_cover: value.no_cover,
            medium_toggle: value.medium_toggle,
            open_proxy: value.open_proxy,
            enable_statistics: value.enable_statistics,
            statistics_recipient: value.statistics_recipient.clone(),
        }
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
        config.storage_paths.network_requester_config = Some(custom_config_path);
    }

    if let Some(override_enabled) = args.enabled {
        config.network_requester.enabled = override_enabled;
    }

    if config.storage_paths.network_requester_config.is_none() {
        config = config.with_default_network_requester_config_path()
    }

    let identity_public_key = load_public_key(
        &config.storage_paths.keys.public_identity_key_file,
        "gateway identity",
    )?;
    let details = initialise_local_network_requester(&config, opts, identity_public_key).await?;
    config.try_save()?;

    args.output.to_stdout(&details);

    Ok(())
}
