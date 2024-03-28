// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliIpPacketRouterClient;
use nym_client_core::cli_helpers::client_switch_gateway::{
    switch_gateway, CommonClientSwitchGatewaysArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

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

pub(crate) async fn execute(args: Args) -> Result<(), IpPacketRouterError> {
    switch_gateway::<CliIpPacketRouterClient, _>(args).await
}
