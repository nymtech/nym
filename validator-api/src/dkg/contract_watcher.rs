// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::Client;
use log::warn;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::time::interval;

pub(crate) struct Watcher<C> {
    client: Client<C>,
    polling_rate: Duration,
}

impl<C> Watcher<C> {
    async fn poll_contract(&self) -> Result<(), DkgError> {
        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval(self.polling_rate);
        loop {
            interval.tick().await;
            if let Err(err) = self.poll_contract().await {
                warn!(
                    "failed to get the current state of the DKG contract - {}",
                    err
                )
            }
        }
    }
}
