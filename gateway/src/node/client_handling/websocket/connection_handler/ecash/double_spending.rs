// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::RequestHandlingError;
use crate::node::client_handling::websocket::connection_handler::ecash::state::SharedState;
use bloomfilter::reexports::bit_vec::BitVec;
use bloomfilter::Bloom;
use log::warn;
use nym_network_defaults::{BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS};
use nym_task::TaskClient;
use nym_validator_client::CoconutApiClient;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub(crate) struct DoubleSpendingDetector {
    spent_serial_numbers: Arc<RwLock<Bloom<String>>>,
    shared_state: SharedState,
}

impl DoubleSpendingDetector {
    pub(crate) fn new(shared_state: SharedState) -> Self {
        let bitmap = [0u8; (BLOOM_BITMAP_SIZE / 8) as usize];
        let bloom_filter =
            Bloom::from_existing(&bitmap, BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS);
        DoubleSpendingDetector {
            spent_serial_numbers: Arc::new(RwLock::new(bloom_filter)),
            shared_state,
        }
    }

    pub(crate) async fn check(&self, serial_number_bs58: &String) -> bool {
        self.spent_serial_numbers
            .read()
            .await
            .check(serial_number_bs58)
    }

    async fn latest_api_endpoints(
        &self,
    ) -> Result<RwLockReadGuard<Vec<CoconutApiClient>>, RequestHandlingError> {
        let epoch_id = self.shared_state.current_epoch_id().await?;
        self.shared_state.api_clients(epoch_id).await
    }

    async fn refresh_bloomfilter(&self) {
        //here be api query and union of different results
        let mut bit_vec = BitVec::from_elem(BLOOM_BITMAP_SIZE as usize, false);
        let api_clients = match self.latest_api_endpoints().await {
            Ok(clients) => clients,
            Err(err) => {
                warn!("failed to obtain current api clients: {err}");
                return;
            }
        };

        for ecash_client in api_clients.deref().iter() {
            match ecash_client.api_client.spent_credentials_filter().await {
                Ok(response) => {
                    if response.bitmap.len() != (BLOOM_BITMAP_SIZE / 8) as usize {
                        //Constant is bit size, len is byte size
                        log::warn!("Validator {} gave us an incompatible bitmap for the double spending detector, we're gonna ignore it", ecash_client.api_client.nym_api.current_url());
                    } else {
                        bit_vec.or(&BitVec::from_bytes(&response.bitmap));
                    }
                }
                Err(e) => {
                    log::warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", ecash_client.api_client.nym_api.current_url(), e);
                }
            }
        }
        let mut filter = self.spent_serial_numbers.write().await;
        *filter = Bloom::from_bit_vec(bit_vec, BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS);
    }

    async fn run(&self, mut shutdown: TaskClient) {
        log::info!("Starting Ecash DoubleSpendingDetector");
        let mut interval = interval(Duration::from_secs(300));

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("ecash_verifier::DoubleSpendingDetector : received shutdown");
                },
                _ = interval.tick() => self.refresh_bloomfilter().await,

            }
        }
    }

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
