// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use nym_credentials_interface::CredentialSpendingData;

use crate::{
    error::Error,
    models::{latest, AvailableBandwidthResponse},
    transceiver::PeerControllerTransceiver,
};

#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    transceiver: PeerControllerTransceiver,
}

impl AppState {
    pub fn new(transceiver: PeerControllerTransceiver) -> Self {
        Self { transceiver }
    }

    pub(crate) async fn available_bandwidth(
        &self,
        ip: IpAddr,
    ) -> Result<AvailableBandwidthResponse, Error> {
        let value = self.transceiver.query_bandwidth(ip).await?;
        let res = latest::InnerAvailableBandwidthResponse::new(value).try_into()?;
        Ok(res)
    }

    pub(crate) async fn topup_bandwidth(
        &self,
        ip: IpAddr,
        credential: CredentialSpendingData,
    ) -> Result<(), Error> {
        self.transceiver
            .topup_bandwidth(ip, Box::new(credential))
            .await?;
        Ok(())
    }
}
