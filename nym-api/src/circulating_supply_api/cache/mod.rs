use rocket::fairing::AdHoc;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::time;

use crate::support::caching::Cache;

use self::data::CirculatingSupplyCacheData;

mod data;
pub(crate) mod refresher;

#[derive(Clone)]
pub(crate) struct CirculatingSupplyCache {
    initialised: Arc<AtomicBool>,
    data: Arc<RwLock<CirculatingSupplyCacheData>>,
}

impl CirculatingSupplyCache {
    fn new() -> CirculatingSupplyCache {
        CirculatingSupplyCache {
            initialised: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(CirculatingSupplyCacheData::new())),
        }
    }

    pub(crate) async fn say_foomp(&self) -> Option<Cache<String>> {
        match time::timeout(Duration::from_millis(100), self.data.read()).await {
            Ok(cache) => Some(cache.circulating_supply.clone()),
            Err(e) => {
                error!("{}", e);
                Some(Cache::new(String::from("0nym")))
            }
        }
    }

    pub(crate) fn stage() -> AdHoc {
        AdHoc::on_ignite("Circulating Supply Cache Stage", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    pub(crate) async fn update(&self, mixmining_temp: validator_client::nyxd::Coin) {
        let mut cache = self.data.write().await;
        cache.circulating_supply = Cache::new(mixmining_temp.to_string());
        self.initialised
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
