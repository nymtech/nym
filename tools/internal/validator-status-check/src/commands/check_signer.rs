// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::helpers::get_signer_status;
use nym_bin_common::output_format::OutputFormat;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,

    /// api address of the specified signer
    #[clap(long)]
    signer: String,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let out = args.output.format(&get_signer_status(&args.signer).await);
    println!("{out}");
    Ok(())
}
