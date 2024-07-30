// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::ecash::error::EcashTicketError;
use crate::node::client_handling::websocket::connection_handler::ecash::state::SharedState;
use crate::node::Storage;
use nym_ecash_double_spending::DoubleSpendingFilter;
use nym_task::TaskClient;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::EcashApiClient;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::time::{interval, Duration};
use tracing::{info, trace, warn};

#[derive(Clone)]
pub(crate) struct DoubleSpendingDetector<S> {
    spent_serial_numbers: Arc<RwLock<DoubleSpendingFilter>>,
    shared_state: SharedState<S>,
}

impl<S> DoubleSpendingDetector<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub(crate) fn new(shared_state: SharedState<S>) -> Self {
        DoubleSpendingDetector {
            spent_serial_numbers: Arc::new(RwLock::new(DoubleSpendingFilter::new_empty_ecash())),
            shared_state,
        }
    }

    pub(crate) async fn check(&self, serial_number: &Vec<u8>) -> bool {
        self.spent_serial_numbers.read().await.check(serial_number)
    }

    async fn latest_api_endpoints(
        &self,
    ) -> Result<RwLockReadGuard<Vec<EcashApiClient>>, EcashTicketError> {
        let epoch_id = self.shared_state.current_epoch_id().await?;
        self.shared_state.api_clients(epoch_id).await
    }

    async fn refresh_bloomfilter(&self) {
        let mut filter_builder = self.spent_serial_numbers.read().await.rebuild();

        let api_clients = match self.latest_api_endpoints().await {
            Ok(clients) => clients,
            Err(err) => {
                warn!("failed to obtain current api clients: {err}");
                return;
            }
        };

        let mut clients = api_clients
            .iter()
            .map(|c| c.api_client.clone())
            .collect::<Vec<_>>();
        clients.shuffle(&mut thread_rng());

        for client in clients {
            match client.nym_api.double_spending_filter_v1().await {
                Ok(response) => {
                    // due to relative big size of the filter, query only one api since all of them should contain
                    // roughly the same data anyway.
                    filter_builder.add_bytes(&response.bitmap);
                    *self.spent_serial_numbers.write().await = filter_builder.build();
                    return;
                }
                Err(err) => {
                    warn!("Validator @ {} could not be reached. There might be a problem with the ecash endpoint: {err}", client.api_url());
                }
            }
        }

        warn!("none of the validators could be reached. the bloomfilter will remain unchanged.");
    }

    async fn run(&self, mut shutdown: TaskClient) {
        info!("Starting Ecash DoubleSpendingDetector");
        let mut interval = interval(Duration::from_secs(600));

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("ecash_verifier::DoubleSpendingDetector : received shutdown");
                },
                _ = interval.tick() => self.refresh_bloomfilter().await,

            }
        }
    }

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
