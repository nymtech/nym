// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::NymNode;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;
use tracing::{debug, info, trace};

mod args;

pub(crate) use args::Args;

pub(crate) async fn execute(mut args: Args) -> Result<(), NymNodeError> {
    trace!("passed arguments: {args:#?}");

    let config_path = args.config.config_path();

    let config = if !config_path.exists() {
        debug!("no configuration file found at '{}'", config_path.display());
        info!("initialising new nym-node");
        if args.deny_init {
            return Err(NymNodeError::ForbiddenInitialisation { config_path });
        }
        let init_only = args.init_only;

        let maybe_custom_mnemonic = args.take_mnemonic();

        let config = args.build_config()?;
        NymNode::initialise(&config, maybe_custom_mnemonic).await?;
        if init_only {
            debug!("returning due to the 'init-only' flag");
            return Ok(());
        }

        config
    } else {
        info!(
            "attempting to load nym-node configuration from {}",
            config_path.display()
        );
        let config = try_load_current_config(config_path).await?;
        args.override_config(config)
    };

    NymNode::new(config).await?.run().await
}
