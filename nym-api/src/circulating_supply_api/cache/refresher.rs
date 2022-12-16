use std::time::Duration;

use tokio::sync::watch;
use validator_client::Client;

use crate::contract_cache::cache::refresher::CacheNotification;

use super::CirculatingSupplyCache;

pub struct CirculatingSupplyCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: CirculatingSupplyCache,
    caching_interval: Duration,

    // Notify listeners that the cache has been updated
    update_notifier: watch::Sender<CacheNotification>,
}
