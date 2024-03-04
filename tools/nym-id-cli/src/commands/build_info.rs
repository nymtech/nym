// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::bin_info_owned;
use nym_bin_common::output_format::OutputFormat;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) fn execute(args: Args) {
    println!("{}", args.output.format(&bin_info_owned!()))
}
