// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::logging::granual_filtered_env;
use crate::throughput_tester::test_mixing_throughput;
use human_repr::HumanDuration;
use indicatif::{ProgressState, ProgressStyle};
use nym_bin_common::logging::default_tracing_fmt_layer;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    config: ConfigArgs,

    #[clap(long, default_value_t = 1)]
    senders: usize,
}

fn init_test_logger() -> anyhow::Result<()> {
    let indicatif_layer = IndicatifLayer::new()
        .with_progress_style(
            ProgressStyle::with_template(
                "{span_child_prefix}{span_fields} -- {span_name} {wide_msg} {elapsed}",
            )?
            .with_key(
                "elapsed",
                |state: &ProgressState, writer: &mut dyn std::fmt::Write| {
                    let _ = writer.write_str(&format!("{}", state.elapsed().human_duration()));
                },
            ),
        )
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

    //
    test_mixing_throughput(args.config.config_path(), args.senders)

    // info!("foo");
    //
    // tokio::runtime::Builder::new_multi_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    //     .block_on(async {
    //         stream::iter((0..20).map(|val| build(val)))
    //             .buffer_unordered(7)
    //             .collect::<Vec<()>>()
    //             .await;
    //     });

    // Ok(())
}
