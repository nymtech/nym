// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::CliIpPacketRouterClient;
use nym_client_core::cli_helpers::client_import_credential::{
    CommonClientImportTicketBookArgs, import_credential,
};
use nym_ip_packet_router::error::IpPacketRouterError;

pub async fn execute(args: CommonClientImportTicketBookArgs) -> Result<(), IpPacketRouterError> {
    import_credential::<CliIpPacketRouterClient, _>(args).await?;
    println!("successfully imported credential!");
    Ok(())
}
