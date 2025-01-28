// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliStatsCollectorClient;
use nym_client_core::cli_helpers::client_switch_gateway::{
    switch_gateway, CommonClientSwitchGatewaysArgs,
};
use nym_statistics_collector::error::StatsCollectorError;

#[derive(clap::Args, Clone, Debug)]
pub struct Args {
    #[command(flatten)]
    common_args: CommonClientSwitchGatewaysArgs,
}

impl AsRef<CommonClientSwitchGatewaysArgs> for Args {
    fn as_ref(&self) -> &CommonClientSwitchGatewaysArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), StatsCollectorError> {
    switch_gateway::<CliStatsCollectorClient, _>(args).await
}
