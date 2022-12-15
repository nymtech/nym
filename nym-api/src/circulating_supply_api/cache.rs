use rocket::fairing::AdHoc;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::{watch, RwLock};
use validator_client::Client;

use crate::contract_cache::cache::refresher::CacheNotification;

#[derive(Clone)]
pub(crate) struct CirculatingSupplyCache {
    initialised: Arc<AtomicBool>,
    inner: Arc<RwLock<CirculatingSupplyCacheInner>>,
}

impl CirculatingSupplyCache {
    fn new() -> CirculatingSupplyCache {
        CirculatingSupplyCache {
            initialised: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(RwLock::new(CirculatingSupplyCacheInner::new())),
        }
    }

    pub(crate) fn say_foomp(&self) -> &'static str {
        "foomp2"
    }

    pub(crate) fn stage() -> AdHoc {
        AdHoc::on_ignite("Circulating Supply Cache Stage", |rocket| async {
            rocket.manage(Self::new())
        })
    }
}

struct CirculatingSupplyCacheInner {}

impl CirculatingSupplyCacheInner {
    pub fn new() -> CirculatingSupplyCacheInner {
        CirculatingSupplyCacheInner {}
    }
}

pub struct CirculatingSupplyCacheRefresher<C> {
    nymd_client: Client<C>,
    cache: CirculatingSupplyCache,
    caching_interval: Duration,

    // Notify listeners that the cache has been updated
    update_notifier: watch::Sender<CacheNotification>,
}
