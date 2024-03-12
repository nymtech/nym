// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliIpPacketRouterClient;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_list_gateways::{
    list_gateways, CommonClientListGatewaysArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

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

pub(crate) async fn execute(args: Args) -> Result<(), IpPacketRouterError> {
    let output = args.output;
    let res = list_gateways::<CliIpPacketRouterClient, _>(args).await?;

    println!("{}", output.format(&res));
    Ok(())
}
