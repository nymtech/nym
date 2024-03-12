// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::CliSocks5Client;
use crate::error::Socks5ClientError;
use nym_client_core::cli_helpers::client_switch_gateway::{
    switch_gateway, CommonClientSwitchGatewaysArgs,
};

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

pub(crate) async fn execute(args: Args) -> Result<(), Socks5ClientError> {
    switch_gateway::<CliSocks5Client, _>(args).await
}
