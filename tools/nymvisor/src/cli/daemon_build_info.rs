// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_load_current_config;
use crate::daemon::Daemon;
use crate::env::Env;
use crate::error::NymvisorError;
use nym_bin_common::output_format::OutputFormat;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    let env = Env::try_read()?;
    let config = try_load_current_config(&env)?;

    let daemon = Daemon::from_config(&config);
    let build_info = daemon.get_build_information()?;

    println!("{}", args.output.format(&build_info));
    Ok(())
}
