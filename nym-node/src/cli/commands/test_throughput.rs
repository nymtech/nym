// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::logging::granual_filtered_env;
use crate::throughput_tester::test_mixing_throughput;
use anyhow::bail;
use humantime_serde::re::humantime;
use nym_bin_common::logging::default_tracing_fmt_layer;
use std::env::temp_dir;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    #[clap(long, default_value_t = 10)]
    senders: usize,

    /// target packet latency, if current value is below threshold, clients will increase their sending rates
    /// and similarly if it's above it, they will decrease it
    #[clap(long, default_value = "15ms", value_parser = humantime::parse_duration)]
    packet_latency_threshold: Duration,

    #[clap(long, default_value_t = 50)]
    starting_sending_batch_size: usize,

    #[clap(long, default_value = "50ms", value_parser = humantime::parse_duration)]
    starting_sending_delay: Duration,

    #[clap(long, short)]
    output_directory: Option<PathBuf>,
}

fn init_test_logger() -> anyhow::Result<()> {
    let indicatif_layer = IndicatifLayer::new()
        .with_span_child_prefix_symbol("↳ ")
        .with_span_child_prefix_indent(" ");

    tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(
            indicatif_layer.get_stderr_writer(),
        ))
        .with(indicatif_layer)
        .with(granual_filtered_env()?)
        .init();
    Ok(())
}

pub fn execute(args: Args) -> anyhow::Result<()> {
    init_test_logger()?;

    let output_dir = match args.output_directory {
        Some(output_dir) => {
            if !output_dir.is_dir() {
                bail!("'{}' is not a directory", output_dir.display());
            }

            output_dir
        }
        None => {
            let now = OffsetDateTime::now_utc().unix_timestamp();
            temp_dir()
                .join("nym-node-throughput-testing")
                .join(now.to_string())
        }
    };

    test_mixing_throughput(
        args.config.config_path(),
        args.senders,
        args.packet_latency_threshold,
        args.starting_sending_batch_size,
        args.starting_sending_delay,
        output_dir,
    )
}
