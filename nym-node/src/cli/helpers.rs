// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::DEFAULT_NYMNODE_ID;
use crate::env::vars::{NYMNODE_CONFIG_PATH_ARG, NYMNODE_ID_ARG};
use clap::builder::ArgPredicate;
use clap::Args;
use nym_node::config::default_config_filepath;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub(crate) struct ConfigArgs {
    /// Id of the nym-node to use
    #[clap(
        long,
        default_value = DEFAULT_NYMNODE_ID,
        default_value_if("config_file", ArgPredicate::IsPresent, None),
        env = NYMNODE_ID_ARG,
        group = "config"
    )]
    id: Option<String>,

    /// Path to a configuration file of this node.
    #[clap(
        long,
        env = NYMNODE_CONFIG_PATH_ARG,
        group = "config"
    )]
    config_file: Option<PathBuf>,
}

impl ConfigArgs {
    pub(crate) fn id(&self) -> &Option<String> {
        &self.id
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        // SAFETY:
        // if `config_file` hasn't been specified, `id` will default to "DEFAULT_NYMNODE_ID",
        // so some value will always be available to use
        #[allow(clippy::unwrap_used)]
        self.config_file
            .clone()
            .unwrap_or(default_config_filepath(self.id.as_ref().unwrap()))
    }
}
