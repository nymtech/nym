use super::ValidatorCache;
use crate::nymd_client::Client;
use anyhow::Result;
use mixnet_contract_common::{MixId, MixNodeDetails, RewardedSetNodeStatus};
use std::{collections::HashMap, sync::atomic::Ordering, time::Duration};
use task::TaskClient;
use tokio::sync::watch;
use tokio::time;
use validator_client::nymd::CosmWasmClient;

pub struct ValidatorCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: ValidatorCache,
    caching_interval: Duration,

    // Notify listeners that the cache has been updated
    update_notifier: watch::Sender<CacheNotification>,
}

impl<C> ValidatorCacheRefresher<C> {
    pub(crate) fn new(
        nymd_client: Client<C>,
        caching_interval: Duration,
        cache: ValidatorCache,
    ) -> Self {
        let (tx, _) = watch::channel(CacheNotification::Start);
        ValidatorCacheRefresher {
            nymd_client,
            cache,
            caching_interval,
            update_notifier: tx,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<CacheNotification> {
        self.update_notifier.subscribe()
    }

    async fn refresh(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let rewarding_params = self.nymd_client.get_current_rewarding_parameters().await?;
        let current_interval = self.nymd_client.get_current_interval().await?.interval;

        let mixnodes = self.nymd_client.get_mixnodes().await?;
        let gateways = self.nymd_client.get_gateways().await?;

        let mix_to_family = self.nymd_client.get_all_family_members().await?;

        let rewarded_set_map = self.get_rewarded_set_map().await;

        let (rewarded_set, active_set) =
            Self::collect_rewarded_and_active_set_details(&mixnodes, &rewarded_set_map);

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len(),
        );

        self.cache
            .update_cache(
                mixnodes,
                gateways,
                rewarded_set,
                active_set,
                rewarding_params,
                current_interval,
                mix_to_family,
            )
            .await;

        if let Err(err) = self.update_notifier.send(CacheNotification::Updated) {
            warn!("Failed to notify validator cache refresh: {err}");
        }

        Ok(())
    }

    async fn get_rewarded_set_map(&self) -> HashMap<MixId, RewardedSetNodeStatus>
    where
        C: CosmWasmClient + Sync + Send,
    {
        self.nymd_client
            .get_rewarded_set_mixnodes()
            .await
            .map(|nodes| nodes.into_iter().collect())
            .unwrap_or_default()
    }

    fn collect_rewarded_and_active_set_details(
        all_mixnodes: &[MixNodeDetails],
        rewarded_set_nodes: &HashMap<MixId, RewardedSetNodeStatus>,
    ) -> (Vec<MixNodeDetails>, Vec<MixNodeDetails>) {
        let mut active_set = Vec::new();
        let mut rewarded_set = Vec::new();

        for mix in all_mixnodes {
            if let Some(status) = rewarded_set_nodes.get(&mix.mix_id()) {
                rewarded_set.push(mix.clone());
                if status.is_active() {
                    active_set.push(mix.clone())
                }
            }
        }

        (rewarded_set, active_set)
    }

    pub(crate) async fn run(&self, mut shutdown: TaskClient)
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mut interval = time::interval(self.caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    tokio::select! {
                        biased;
                        _ = shutdown.recv() => {
                            trace!("ValidatorCacheRefresher: Received shutdown");
                        }
                        ret = self.refresh() => {
                            if let Err(err) = ret {
                                error!("Failed to refresh validator cache - {err}");
                            } else {
                                // relaxed memory ordering is fine here. worst case scenario network monitor
                                // will just have to wait for an additional backoff to see the change.
                                // And so this will not really incur any performance penalties by setting it every loop iteration
                                self.cache.initialised.store(true, Ordering::Relaxed)
                            }
                        }
                    }
                }
                _ = shutdown.recv() => {
                    trace!("ValidatorCacheRefresher: Received shutdown");
                }
            }
        }
    }
}

// The cache can emit notifications to listeners about the current state
#[derive(Debug, PartialEq, Eq)]
pub enum CacheNotification {
    Start,
    Updated,
}
