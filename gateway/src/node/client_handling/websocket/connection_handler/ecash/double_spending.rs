// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use bloomfilter::reexports::bit_vec::BitVec;
use bloomfilter::Bloom;
use nym_network_defaults::{BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS};
use nym_task::TaskClient;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::CoconutApiClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
#[derive(Clone)]
pub(crate) struct DoubleSpendingDetector {
    spent_serial_numbers: Arc<RwLock<Bloom<String>>>,
    ecash_clients: Arc<RwLock<(EpochId, Vec<CoconutApiClient>)>>,
}

impl DoubleSpendingDetector {
    pub(crate) fn new() -> Self {
        let bitmap = [0u8; (BLOOM_BITMAP_SIZE / 8) as usize];
        let bloom_filter =
            Bloom::from_existing(&bitmap, BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS);
        DoubleSpendingDetector {
            spent_serial_numbers: Arc::new(RwLock::new(bloom_filter)),
            ecash_clients: Default::default(),
        }
    }

    pub(crate) async fn check(&self, serial_number_bs58: &String) -> bool {
        self.spent_serial_numbers
            .read()
            .await
            .check(serial_number_bs58)
    }

    async fn update(&self) {
        //here be api query and union of different results
        let mut bit_vec = BitVec::from_elem(BLOOM_BITMAP_SIZE as usize, false);
        for ecash_client in &self.ecash_clients.read().await.1 {
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
    pub(crate) async fn update_api_client(
        &self,
        epoch_id: EpochId,
        api_client: Vec<CoconutApiClient>,
    ) {
        let mut current_clients = self.ecash_clients.write().await;
        if epoch_id >= current_clients.0 {
            current_clients.1 = api_client;
        }
        drop(current_clients);
        self.update().await;
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
                _ = interval.tick() => self.update().await,

            }
        }
    }

    pub(crate) fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });
    }
}
