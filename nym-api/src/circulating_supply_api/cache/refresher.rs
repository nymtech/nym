use super::CirculatingSupplyCache;
use crate::support::{caching::CacheNotification, nyxd::Client};
use anyhow::Result;
use cosmwasm_std::Addr;
use std::sync::atomic::Ordering;
use std::time::Duration;
use task::TaskClient;
use tokio::sync::watch;
use tokio::time;
use validator_client::nyxd::CosmWasmClient;

pub(crate) struct CirculatingSupplyCacheRefresher<C> {
    nyxd_client: Client<C>,
    cache: CirculatingSupplyCache,
    caching_interval: Duration,

    // Notify listeners that the cache has been updated
    update_notifier: watch::Sender<CacheNotification>,
}

impl<C> CirculatingSupplyCacheRefresher<C> {
    pub(crate) fn new(
        nyxd_client: Client<C>,
        cache: CirculatingSupplyCache,
        caching_interval: Duration,
    ) -> Self {
        let (tx, _) = watch::channel(CacheNotification::Start);

        CirculatingSupplyCacheRefresher {
            nyxd_client,
            cache,
            caching_interval,
            update_notifier: tx,
        }
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<CacheNotification> {
        self.update_notifier.subscribe()
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

    async fn refresh(&self) -> Result<()>
    where
        C: CosmWasmClient + Sync + Send,
    {
        let mixmining_temp = self
            .nyxd_client
            .get_balance(Addr::unchecked("n1299fhjdafamwc2gha723nkkewvu56u5xn78t9j").into())
            .await?;

        info!(
            "Updating circulating supply cache. Circulating supply is now: {} unym",
            mixmining_temp,
        );

        self.cache.update(mixmining_temp).await;

        if let Err(err) = self.update_notifier.send(CacheNotification::Updated) {
            warn!("Failed to notify circulating supply cache refresh: {err}");
        }

        Ok(())
    }
}
