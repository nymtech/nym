// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::CliIpPacketRouterClient;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_show_ticketbooks::{
    show_ticketbooks, CommonShowTicketbooksArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

#[derive(clap::Args)]
pub struct Args {
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

pub async fn execute(args: Args) -> Result<(), IpPacketRouterError> {
    let output = args.output;
    let res = show_ticketbooks::<CliIpPacketRouterClient, _>(args).await?;

    println!("{}", output.format(&res));
    Ok(())
}
