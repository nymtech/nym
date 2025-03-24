// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::throughput_test::test_mixing_throughput;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    #[clap(long, default_value_t = 10)]
    senders: usize,
}

pub fn execute(args: Args) -> anyhow::Result<()> {
    test_mixing_throughput(args.config.config_path(), args.senders)
}
