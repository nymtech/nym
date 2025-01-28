// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_add_gateway::{add_gateway, CommonClientAddGatewayArgs};
use nym_statistics_collector::error::StatsCollectorError;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientAddGatewayArgs,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonClientAddGatewayArgs> for Args {
    fn as_ref(&self) -> &CommonClientAddGatewayArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), StatsCollectorError> {
    let output = args.output;
    let res = add_gateway::<CliStatsCollectorClient, _>(args, None).await?;

    println!("{}", output.format(&res));
    Ok(())
}
