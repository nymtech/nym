// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_show_ticketbooks::{
    show_ticketbooks, CommonShowTicketbooksArgs,
};

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

pub async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    let output = args.output;
    let res = show_ticketbooks::<CliSocks5Client, _>(args).await?;

    println!("{}", output.format(&res));
    Ok(())
}
