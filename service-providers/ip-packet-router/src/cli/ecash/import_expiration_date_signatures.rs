// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliIpPacketRouterClient;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::{
    import_expiration_date_signatures, CommonClientImportExpirationDateSignaturesArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

pub(crate) async fn execute(
    args: CommonClientImportExpirationDateSignaturesArgs,
) -> Result<(), IpPacketRouterError> {
    import_expiration_date_signatures::<CliIpPacketRouterClient, _>(args).await?;
    println!("successfully imported expiration date signatures!");
    Ok(())
}
