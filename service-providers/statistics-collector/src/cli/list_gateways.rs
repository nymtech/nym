// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_list_gateways::{
    list_gateways, CommonClientListGatewaysArgs,
};
use nym_statistics_collector::error::StatsCollectorError;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonClientListGatewaysArgs,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonClientListGatewaysArgs> for Args {
    fn as_ref(&self) -> &CommonClientListGatewaysArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), StatsCollectorError> {
    let output = args.output;
    let res = list_gateways::<CliStatsCollectorClient, _>(args).await?;

    println!("{}", output.format(&res));
    Ok(())
}
