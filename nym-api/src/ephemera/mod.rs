// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use clap::Parser;
use serde_derive::{Deserialize, Serialize};

pub mod application;
pub mod epoch;
pub mod metrics;
pub mod peers;
pub mod reward;

#[derive(Parser, Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Args {
    #[clap(long)]
    pub metrics_db_path: String,
    #[clap(long, default_value = "5")]
    pub metrics_collector_interval_seconds: i64,
    #[clap(long, default_value = "20")]
    pub epoch_duration_seconds: u64,
    #[clap(long)]
    pub smart_contract_url: String,
    #[clap(long)]
    pub ephemera_config: String,
    #[clap(long)]
    pub nym_api_id: String,
    #[clap(long, default_value = "1")]
    pub block_polling_interval_seconds: u64,
    #[clap(long, default_value = "60")]
    pub block_polling_max_attempts: u64,
}

#[derive(Parser, Debug, Clone)]
pub struct ContractArgs {
    #[clap(long)]
    pub db_path: String,
    #[clap(long, default_value = "60")]
    pub epoch_duration_seconds: u64,
    #[clap(long)]
    pub url: String,
    #[clap(long)]
    pub ephemera_config: String,
}
