// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use nym_credentials_interface::CredentialSpendingData;

use crate::{error::Error, transceiver::PeerControllerTransceiver};

#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    transceiver: PeerControllerTransceiver,
}

impl AppState {
    pub fn new(transceiver: PeerControllerTransceiver) -> Self {
        Self { transceiver }
    }

    pub(crate) async fn available_bandwidth(&self, ip: IpAddr) -> Result<i64, Error> {
        self.transceiver.query_bandwidth(ip).await
    }

    pub(crate) async fn topup_bandwidth(
        &self,
        ip: IpAddr,
        credential: CredentialSpendingData,
    ) -> Result<i64, Error> {
        self.transceiver
            .topup_bandwidth(ip, Box::new(credential))
            .await
    }
}
