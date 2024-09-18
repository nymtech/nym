// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliIpPacketRouterClient;
use nym_client_core::cli_helpers::client_import_master_verification_key::{
    import_master_verification_key, CommonClientImportMasterVerificationKeyArgs,
};
use nym_ip_packet_router::error::IpPacketRouterError;

pub(crate) async fn execute(
    args: CommonClientImportMasterVerificationKeyArgs,
) -> Result<(), IpPacketRouterError> {
    import_master_verification_key::<CliIpPacketRouterClient, _>(args).await?;
    println!("successfully imported master verification key!");
    Ok(())
}
