// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::bonding_information::BondingInformationV1;
use crate::node::NymNode;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;
use std::fs;
use tracing::{debug, info, trace};

mod args;

pub(crate) use args::Args;

pub(crate) async fn execute(mut args: Args) -> Result<(), NymNodeError> {
    trace!("passed arguments: {args:#?}");

    let config_path = args.config.config_path();
    let output = args.output;
    let bonding_info_path = args.bonding_information_output.clone();

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

    let nym_node = NymNode::new(config).await?;

    // if requested, write bonding info
    if let Some(bonding_info_path) = bonding_info_path {
        info!(
            "writing bonding information to '{}'",
            bonding_info_path.display()
        );
        let info = BondingInformationV1::from_data(
            nym_node.ed25519_identity_key(),
            nym_node.x25519_sphinx_key(),
        );
        let data = output.format(&info);
        fs::write(&bonding_info_path, data).map_err(|source| {
            NymNodeError::BondingInfoWriteFailure {
                path: bonding_info_path,
                source,
            }
        })?;
    }

    nym_node.run().await
}
