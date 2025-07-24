// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use crate::{error::Error, models::AvailableBandwidth, transceiver::PeerControllerTransceiver};

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
    ) -> Result<AvailableBandwidth, Error> {
        let value = self.transceiver.query_bandwidth(ip).await?;
        Ok(AvailableBandwidth { value })
    }
}
