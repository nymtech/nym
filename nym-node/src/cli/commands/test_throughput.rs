// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::logging::granual_filtered_env;
use crate::throughput_tester::test_mixing_throughput;
use humantime_serde::re::humantime;
use indicatif::ProgressStyle;
use nym_bin_common::logging::default_tracing_fmt_layer;
use std::time::Duration;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    #[clap(long, default_value_t = 3)]
    senders: usize,

    #[clap(long, default_value = "15ms", value_parser = humantime::parse_duration)]
    packet_latency_threshold: Duration,
}

fn init_test_logger() -> anyhow::Result<()> {
    let indicatif_layer = IndicatifLayer::new()
        .with_progress_style(ProgressStyle::with_template(
            "{span_child_prefix}{spinner} {span_fields} -- {span_name} {wide_msg}",
        )?)
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

    test_mixing_throughput(
        args.config.config_path(),
        args.senders,
        args.packet_latency_threshold,
    )
}
