// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_statistics_collector::error::StatsCollectorError;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_show_ticketbooks::{
    show_ticketbooks, CommonShowTicketbooksArgs,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonShowTicketbooksArgs,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonShowTicketbooksArgs> for Args {
    fn as_ref(&self) -> &CommonShowTicketbooksArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), StatsCollectorError> {
    let output = args.output;
    let res = show_ticketbooks::<CliStatsCollectorClient, _>(args).await?;

    println!("{}", output.format(&res));
    Ok(())
}
