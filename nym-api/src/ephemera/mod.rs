// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use clap::Parser;
use serde_derive::{Deserialize, Serialize};

pub(crate) mod application;
pub(crate) mod client;
pub(crate) mod epoch;
pub(crate) mod error;
pub(crate) mod metrics;
pub(crate) mod peers;
pub(crate) mod reward;

#[derive(Parser, Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Args {
    #[clap(long)]
    pub ephemera_config: String,
    #[clap(long, default_value = "1")]
    pub block_polling_interval_seconds: u64,
    #[clap(long, default_value = "60")]
    pub block_polling_max_attempts: u64,
}
