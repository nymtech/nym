// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::bin_info_owned;
use nym_bin_common::output_format::OutputFormat;
use tracing::debug;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    debug!("args: {args:#?}");
    println!("{}", args.output.format(&bin_info_owned!()));
    Ok(())
}
