// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliNativeClient;
use crate::error::ClientError;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_show_ticketbooks::{
    show_ticketbooks, CommonShowTicketbooksArgs,
};
use nym_client_core::cli_helpers::database_key::DatabaseKeyArgs;

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    common_args: CommonShowTicketbooksArgs,

    #[command(flatten)]
    database_key: DatabaseKeyArgs,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl AsRef<CommonShowTicketbooksArgs> for Args {
    fn as_ref(&self) -> &CommonShowTicketbooksArgs {
        &self.common_args
    }
}

pub(crate) async fn execute(args: Args) -> Result<(), ClientError> {
    let res =
        show_ticketbooks::<CliNativeClient, _>(&args, args.database_key.database_key.as_ref())
            .await?;

    println!("{}", args.output.format(&res));
    Ok(())
}
