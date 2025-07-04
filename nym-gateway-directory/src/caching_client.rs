// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{FutureExt, StreamExt, stream::FuturesUnordered};
use nym_offline_monitor::ConnectivityHandle;
pub use nym_sdk::mixnet::NodeIdentity;
use strum::IntoEnumIterator;
use tokio::sync::Mutex;

use crate::{
    Config, Country, Error, Gateway, GatewayClient, GatewayList, GatewayType, error::Result,
};

#[derive(Clone)]
pub struct CachingGatewayClient {
    inner: Arc<Mutex<CachingGatewayClientInner>>,
}

impl CachingGatewayClient {
    pub fn new(
        gateway_client: GatewayClient,
        connectivity_handle: Option<ConnectivityHandle>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CachingGatewayClientInner {
                gateway_client,
                connectivity_handle,
                cached_gateways: Default::default(),
                cached_countries: Default::default(),
            })),
        }
    }

    pub async fn new_from_existing(existing_client: &CachingGatewayClient) -> Self {
        let inner = existing_client.inner.lock().await;
        Self {
            inner: Arc::new(Mutex::new(CachingGatewayClientInner {
                gateway_client: inner.gateway_client.clone(),
                connectivity_handle: inner.connectivity_handle.clone(),
                cached_gateways: inner.cached_gateways.clone(),
                cached_countries: inner.cached_countries.clone(),
            })),
        }
    }

    pub async fn update_client(&self, new_client: GatewayClient) {
        self.inner.lock().await.gateway_client = new_client;
    }

    pub async fn set_connectivity_handle(&self, connectivity_handle: ConnectivityHandle) {
        self.inner.lock().await.connectivity_handle = Some(connectivity_handle);
    }

    pub async fn get_config(&self) -> Config {
        self.inner.lock().await.gateway_client.get_config()
    }

    pub async fn refresh_all(&self) {
        self.inner.lock().await.refresh_all().await
    }

    pub async fn force_refresh_all(&self) {
        self.inner.lock().await.force_refresh_all().await
    }

    pub async fn lookup_gateways(&self, gw_type: GatewayType) -> Result<GatewayList> {
        self.inner.lock().await.lookup_gateways(gw_type).await
    }

    pub async fn lookup_countries(&self, gw_type: GatewayType) -> Result<Vec<Country>> {
        self.inner.lock().await.lookup_countries(gw_type).await
    }

    pub async fn lookup_gateway_ip(&self, gateway_identity: &str) -> Result<IpAddr> {
        self.inner
            .lock()
            .await
            .lookup_gateway_ip(gateway_identity)
            .await
    }
}

/// A caching client that wraps around the `GatewayClient` and caches the results of
/// `lookup_gateways` and `lookup_countries` calls.
struct CachingGatewayClientInner {
    // The underlying client that actually does the work
    gateway_client: GatewayClient,

    // The connectivity handle to check if we are online
    connectivity_handle: Option<ConnectivityHandle>,

    // The cached gateways and their last updated time
    cached_gateways: HashMap<GatewayType, (GatewayList, Instant)>,

    // The cached countries and their last updated time
    cached_countries: HashMap<GatewayType, (Vec<Country>, Instant)>,
}

enum LookupResult {
    Gateways(Result<GatewayList>),
    Countries(Result<Vec<Country>>),
}

impl CachingGatewayClientInner {
    /// The maximum age of the cache before it is considered stale.
    const MAX_CACHE_AGE: Duration = Duration::from_secs(5 * 60);

    async fn check_offline(&self) -> bool {
        if let Some(connectivity_handle) = &self.connectivity_handle {
            if connectivity_handle.connectivity().await.is_offline() {
                return true;
            }
        }
        false
    }

    pub async fn refresh_all(&mut self) {
        tracing::info!("Refreshing all gateways and countries");
        self.refresh(
            self.get_stale_gateway_list_types(),
            self.get_stale_country_list_types(),
        )
        .await;
    }

    pub async fn force_refresh_all(&mut self) {
        tracing::info!("Forcing refresh of all gateways and countries");
        self.refresh(GatewayType::iter().collect(), GatewayType::iter().collect())
            .await;
    }

    fn get_stale_gateway_list_types(&self) -> Vec<GatewayType> {
        let mut stale_gw_types = Vec::new();
        for gw_type in GatewayType::iter() {
            if !self.is_gateways_current(&gw_type) {
                stale_gw_types.push(gw_type.clone());
            }
        }
        stale_gw_types
    }

    fn get_stale_country_list_types(&self) -> Vec<GatewayType> {
        let mut stale_gw_types = Vec::new();
        for gw_type in GatewayType::iter() {
            if !self.is_countries_current(&gw_type) {
                stale_gw_types.push(gw_type.clone());
            }
        }
        stale_gw_types
    }

    async fn refresh(&mut self, gw_list_types: Vec<GatewayType>, country_types: Vec<GatewayType>) {
        if self.check_offline().await {
            tracing::warn!("Not refreshing gateways and countries because we are not connected");
            return;
        }

        tracing::info!(
            "Refreshing gateway lists: {gw_list_types:?}, country lists: {country_types:?}"
        );
        let mut tasks = FuturesUnordered::new();

        for gw_type in country_types {
            let client = self.gateway_client.clone();
            tasks.push(
                async move {
                    let res = client.lookup_countries(gw_type.clone()).await;
                    (gw_type, LookupResult::Countries(res))
                }
                .boxed(),
            );
        }
        for gw_type in gw_list_types {
            let client = self.gateway_client.clone();
            tasks.push(
                async move {
                    let res = client.lookup_gateways(gw_type.clone()).await;
                    (gw_type, LookupResult::Gateways(res))
                }
                .boxed(),
            );
        }

        while let Some((gw_type, res)) = tasks.next().await {
            match res {
                LookupResult::Gateways(r) => match r {
                    Ok(ref refreshed_gateways) => {
                        tracing::info!("Refreshed gateways for {gw_type:?}");
                        self.cached_gateways.insert(
                            gw_type.clone(),
                            (refreshed_gateways.clone(), Instant::now()),
                        );
                    }
                    Err(err) => {
                        tracing::warn!("Failed to refresh gateways for {gw_type:?}: {err}");
                    }
                },
                LookupResult::Countries(r) => match r {
                    Ok(ref refreshed_countries) => {
                        tracing::info!("Refreshed countries for {gw_type:?}");
                        self.cached_countries.insert(
                            gw_type.clone(),
                            (refreshed_countries.clone(), Instant::now()),
                        );
                    }
                    Err(err) => {
                        tracing::warn!("Failed to refresh countries for {gw_type:?}: {err}");
                    }
                },
            }
        }
    }

    fn is_countries_current(&self, gw_type: &GatewayType) -> bool {
        if let Some((_, last_updated)) = self.cached_countries.get(gw_type) {
            last_updated.elapsed() < Self::MAX_CACHE_AGE
        } else {
            false
        }
    }

    fn is_gateways_current(&self, gw_type: &GatewayType) -> bool {
        if let Some((_, last_updated)) = self.cached_gateways.get(gw_type) {
            last_updated.elapsed() < Self::MAX_CACHE_AGE
        } else {
            false
        }
    }

    async fn refresh_countries(&mut self, gw_type: GatewayType) -> Result<Vec<Country>> {
        if let Some((countries, last_updated)) = self.cached_countries.get(&gw_type) {
            if last_updated.elapsed() < Self::MAX_CACHE_AGE {
                return Ok(countries.clone());
            }
        }
        self.force_refresh_countries(gw_type).await
    }

    async fn force_refresh_countries(&mut self, gw_type: GatewayType) -> Result<Vec<Country>> {
        if self.check_offline().await {
            tracing::warn!("Not refreshing countries because we are not connected");
            return Err(Error::Offline);
        }
        let refreshed_countries = self
            .gateway_client
            .lookup_countries(gw_type.clone())
            .await?;
        self.cached_countries.insert(
            gw_type.clone(),
            (refreshed_countries.clone(), Instant::now()),
        );
        Ok(refreshed_countries)
    }

    async fn refresh_gateways(&mut self, gw_type: GatewayType) -> Result<GatewayList> {
        if let Some((gw_list, last_updated)) = self.cached_gateways.get(&gw_type) {
            if last_updated.elapsed() < Self::MAX_CACHE_AGE {
                return Ok(gw_list.clone());
            }
        }
        self.force_refresh_gateways(gw_type).await
    }

    async fn force_refresh_gateways(&mut self, gw_type: GatewayType) -> Result<GatewayList> {
        if self.check_offline().await {
            tracing::warn!("Not refreshing countries because we are not connected");
            return Err(Error::Offline);
        }
        let refreshed_gateways = self.gateway_client.lookup_gateways(gw_type.clone()).await?;
        self.cached_gateways.insert(
            gw_type.clone(),
            (refreshed_gateways.clone(), Instant::now()),
        );
        Ok(refreshed_gateways)
    }

    async fn lookup_gateways(&mut self, gw_type: GatewayType) -> Result<GatewayList> {
        let refresh_result = self.refresh_gateways(gw_type.clone()).await;

        // Regardless of if we managed to refresh the cache, we return the cached gateways if they
        // exist. They should be the most recent one we can muster
        if let Some((gateways, _)) = self.cached_gateways.get(&gw_type) {
            Ok(gateways.clone())
        } else {
            refresh_result
        }
    }

    async fn lookup_countries(&mut self, gw_type: GatewayType) -> Result<Vec<Country>> {
        let refresh_result = self.refresh_countries(gw_type.clone()).await;

        // Regardless of if we managed to refresh the cache, we return the cached countries if they
        // exist. They should be the most recent one we can muster
        if let Some((countries, _)) = self.cached_countries.get(&gw_type) {
            Ok(countries.clone())
        } else {
            refresh_result
        }
    }

    async fn lookup_gateway_ip(&mut self, gateway_identity: &str) -> Result<IpAddr> {
        // If we have a populated list of gateways, we should always be able to find the IP there.
        if let Ok(identity) = NodeIdentity::from_base58_string(gateway_identity) {
            for (_, (gateways, _)) in self.cached_gateways.iter() {
                if let Some(ip) = gateways
                    .node_with_identity(&identity)
                    .and_then(Gateway::lookup_ip)
                {
                    return Ok(ip);
                }
            }
        } else {
            tracing::warn!("Failed to parse gateway identity: {gateway_identity}");
        }

        // Fallback
        tracing::warn!("Using fallback to lookup gateway IP");
        self.gateway_client
            .lookup_gateway_ip(gateway_identity)
            .await
    }
}
