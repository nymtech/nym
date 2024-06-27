// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::cli_helpers::client_add_gateway::{add_gateway, CommonClientAddGatewayArgs};

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

pub(crate) async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    let user_agent = nym_bin_common::bin_info!().into();
    let output = args.output;
    let res = add_gateway::<CliSocks5Client, _>(args, Some(user_agent)).await?;

    println!("{}", output.format(&res));
    Ok(())
}
