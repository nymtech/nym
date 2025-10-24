// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CredentialSpendingData;
use nym_wireguard_private_metadata_server::PeerControllerTransceiver;
use nym_wireguard_private_metadata_shared::error::MetadataError;
use std::net::IpAddr;

#[derive(Clone, axum::extract::FromRef)]
pub struct AppStateV1 {
    transceiver: PeerControllerTransceiver,
}

impl AppStateV1 {
    pub fn new(transceiver: PeerControllerTransceiver) -> Self {
        Self { transceiver }
    }

    pub async fn available_bandwidth(&self, ip: IpAddr) -> Result<i64, MetadataError> {
        self.transceiver.query_bandwidth(ip).await
    }

    // Top up with a credential and return the afterwards available bandwidth
    pub async fn topup_bandwidth(
        &self,
        ip: IpAddr,
        credential: CredentialSpendingData,
    ) -> Result<i64, MetadataError> {
        self.transceiver
            .topup_bandwidth(ip, Box::new(credential))
            .await
    }
}
