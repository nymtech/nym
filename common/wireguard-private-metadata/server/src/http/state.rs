// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use nym_credentials_interface::CredentialSpendingData;

use crate::transceiver::PeerControllerTransceiver;
use nym_wireguard_private_metadata_shared::error::MetadataError;

#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    transceiver: PeerControllerTransceiver,
}

impl AppState {
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
