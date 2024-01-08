// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::Args;
use nym_bin_common::bin_info_owned;
use nym_bin_common::output_format::OutputFormat;

#[derive(Args)]
pub(crate) struct BuildInfo {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) fn execute(args: BuildInfo) {
    println!("{}", args.output.format(&bin_info_owned!()))
}
