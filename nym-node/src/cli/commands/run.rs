// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::env::vars::{NYMNODE_DENY_INIT_ARG, NYMNODE_INIT_ONLY_ARG, NYMNODE_MODE_ARG};
use crate::node::NymNode;
use nym_node::config::{Config, NodeMode};
use nym_node::error::NymNodeError;
use tracing::{debug, trace};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    /// Forbid a new node from being initialised if configuration file for the provided specification doesn't already exist
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_DENY_INIT_ARG,
        conflicts_with = "init_only"
    )]
    deny_init: bool,

    /// If this is a brand new nym-node, specify whether it should only be initialised without actually running the subprocesses.
    #[clap(
        long,
        default_value_t = false,
        env = NYMNODE_INIT_ONLY_ARG,
        conflicts_with = "deny_init"
    )]
    init_only: bool,

    /// Specifies the current mode of this nym-node.
    #[clap(
        long,
        value_enum,
        default_value_t = NodeMode::Mixnode,
        env = NYMNODE_MODE_ARG
    )]
    mode: NodeMode,
    //     mixnode-args
    // entry-gateway-args
    // exit-gateway args
}

impl Args {
    fn build_config(&self) -> Config {
        todo!()
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), NymNodeError> {
    trace!("passed arguments: {args:#?}");

    let config_path = args.config.config_path();

    let config = if !config_path.exists() {
        if args.deny_init {
            return Err(NymNodeError::ForbiddenInitialisation { config_path });
        }
        let config = args.build_config();
        NymNode::initialise(&config)?;
        if args.init_only {
            debug!("returning due to the 'init-only' flag");
            return Ok(());
        }

        config
    } else {
        Config::read_from_toml_file(config_path)?
    };

    NymNode::new(config)?.run().await
}
