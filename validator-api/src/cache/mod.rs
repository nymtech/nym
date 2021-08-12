// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd_client::Client;
use anyhow::Result;
use config::defaults::VALIDATOR_API_VERSION;
use mixnet_contract::{GatewayBond, MixNodeBond};
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time;
use validator_client::nymd::CosmWasmClient;

pub(crate) mod routes;

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,
}

#[derive(Clone)]
pub struct ValidatorCache {
    inner: Arc<ValidatorCacheInner>,
}

struct ValidatorCacheInner {
    initialised: AtomicBool,
    mixnodes: RwLock<Cache<Vec<MixNodeBond>>>,
    gateways: RwLock<Cache<Vec<GatewayBond>>>,
}

#[derive(Default, Serialize, Clone)]
pub struct Cache<T> {
    value: T,
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

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<C> ValidatorCacheRefresher<C> {
    pub(crate) fn new(
        nymd_client: Client<C>,
        caching_interval: Duration,
        cache: ValidatorCache,
    ) -> Self {
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
        }
    }

    async fn refresh_cache(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync,
    {
        let (mixnodes, gateways) = tokio::try_join!(
            self.nymd_client.get_mixnodes(),
            self.nymd_client.get_gateways()
        )?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len()
        );

        self.cache.update_cache(mixnodes, gateways).await;

        Ok(())
    }

    pub(crate) async fn run(&self)
    where
        C: CosmWasmClient + Sync,
    {
        let mut interval = time::interval(self.caching_interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.refresh_cache().await {
                error!("Failed to refresh validator cache - {}", err);
            } else {
                // relaxed memory ordering is fine here. worst case scenario network monitor
                // will just have to wait for an additional backoff to see the change.
                // And so this will not really incur any performance penalties by setting it every loop iteration
                self.cache.inner.initialised.store(true, Ordering::Relaxed)
            }
        }
    }
}

impl ValidatorCache {
    fn new() -> Self {
        ValidatorCache {
            inner: Arc::new(ValidatorCacheInner::new()),
        }
    }

    pub fn stage() -> AdHoc {
        AdHoc::on_ignite("Validator Cache Stage", |rocket| async {
            rocket.manage(Self::new()).mount(
                VALIDATOR_API_VERSION,
                routes![routes::get_mixnodes, routes::get_gateways],
            )
        })
    }

    async fn update_cache(&self, mixnodes: Vec<MixNodeBond>, gateways: Vec<GatewayBond>) {
        self.inner.mixnodes.write().await.set(mixnodes);
        self.inner.gateways.write().await.set(gateways);
    }

    pub async fn mixnodes(&self) -> Cache<Vec<MixNodeBond>> {
        self.inner.mixnodes.read().await.clone()
    }

    pub async fn gateways(&self) -> Cache<Vec<GatewayBond>> {
        self.inner.gateways.read().await.clone()
    }

    pub fn initialised(&self) -> bool {
        self.inner.initialised.load(Ordering::Relaxed)
    }
}

impl ValidatorCacheInner {
    fn new() -> Self {
        ValidatorCacheInner {
            initialised: AtomicBool::new(false),
            mixnodes: RwLock::new(Cache::default()),
            gateways: RwLock::new(Cache::default()),
        }
    }
}
