// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;
use mixnet_contract::{GatewayBond, MixNodeBond};
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use validator_client::Client;

pub(crate) mod routes;

#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<ValidatorCacheInner>,
}

struct ValidatorCacheInner {
    mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,
    gateways: RwLock<Cache<Vec<GatewayBond>>>,
    validator_client: Client,
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
    #[allow(dead_code)]
    as_at: u64,
}

impl<T: Clone> Cache<T> {
    fn set(&mut self, value: T) {
        self.value = value;
        self.as_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[allow(dead_code)]
    pub fn get(&self) -> T {
        self.value.clone()
    }
}

impl ValidatorCache {
    async fn init(validators_rest_uris: Vec<String>, mixnet_contract: String) -> Self {
        ValidatorCache {
            inner: Arc::new(ValidatorCacheInner::init(
                validators_rest_uris,
                mixnet_contract,
            )),
        }
    }

    pub fn stage(validators_rest_uris: Vec<String>, mixnet_contract: String) -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket
                .manage(Self::init(validators_rest_uris, mixnet_contract))
                .mount("/v1", routes![routes::get_mixnodes, routes::get_gateways])
        })
    }

    pub async fn refresh_cache(&self) -> Result<()> {
        let (mixnodes, gateways) = tokio::join!(
            self.inner.validator_client.get_mix_nodes(),
            self.inner.validator_client.get_gateways()
        );

        self.inner.mixnodes.write().await.set(mixnodes?);
        self.inner.gateways.write().await.set(gateways?);

        Ok(())
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.mixnodes.read().await.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.inner.gateways.read().await.clone()
    }

    pub async fn run(&self, caching_interval: Duration) {
        let mut interval = time::interval(caching_interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.refresh_cache().await {
                error!("Failed to refresh validator cache - {}", err);
            }
        }
    }
}

impl ValidatorCacheInner {
    fn init(validators_rest_uris: Vec<String>, mixnet_contract: String) -> Self {
        let config = validator_client::Config::new(validators_rest_uris, mixnet_contract);
        let validator_client = validator_client::Client::new(config);

        ValidatorCacheInner {
            mixnodes: RwLock::new(Cache::default()),
            gateways: RwLock::new(Cache::default()),
            validator_client,
        }
    }
}
