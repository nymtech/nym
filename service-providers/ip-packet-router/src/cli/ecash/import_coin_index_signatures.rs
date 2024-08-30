// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliIpPacketRouterClient;
use crate::error::ClientError;
use nym_client_core::cli_helpers::client_import_coin_index_signatures::{
    import_coin_index_signatures, CommonClientImportCoinIndexSignaturesArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

pub(crate) async fn execute(
    args: CommonClientImportCoinIndexSignaturesArgs,
) -> Result<(), IpPacketRouterError> {
    import_coin_index_signatures::<CliIpPacketRouterClient, _>(args).await?;
    println!("successfully imported coin index signatures!");
    Ok(())
}
