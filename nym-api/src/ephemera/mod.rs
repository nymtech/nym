// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use clap::Parser;
use ephemera::cli::init::Cmd;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) mod application;
pub(crate) mod client;
pub(crate) mod epoch;
pub(crate) mod error;
pub(crate) mod metrics;
pub(crate) mod peers;
pub(crate) mod reward;

#[derive(Parser, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Args {
    #[clap(skip)]
    pub ephemera_config: PathBuf,
    #[command(flatten)]
    #[serde(skip)]
    pub cmd: Cmd,
    #[clap(skip)]
    #[serde(skip, default = "default_block_polling_interval_seconds")]
    pub block_polling_interval_seconds: u64,
    #[clap(skip)]
    #[serde(skip, default = "default_block_polling_max_attempts")]
    pub block_polling_max_attempts: u64,
}

fn default_block_polling_interval_seconds() -> u64 {
    1
}

fn default_block_polling_max_attempts() -> u64 {
    60
}

impl Default for Args {
    fn default() -> Self {
        Args {
            ephemera_config: Default::default(),
            cmd: Default::default(),
            block_polling_interval_seconds: default_block_polling_interval_seconds(),
            block_polling_max_attempts: default_block_polling_max_attempts(),
        }
    }
}
