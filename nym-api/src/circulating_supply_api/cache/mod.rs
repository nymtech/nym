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

/// A cache for the circulating supply of the network. Circulating supply is calculated by
/// taking the initial supply of 1bn coins, and subtracting the amount of coins that are
/// in the staking pool, company accounts, and tied up in vesting.
///
/// The cache is quite simple and does not include an update listener that the other caches have.
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

    pub(crate) async fn get_circulating_supply(&self) -> Option<Cache<String>> {
        match time::timeout(Duration::from_millis(100), self.data.read()).await {
            Ok(cache) => Some(cache.circulating_supply.clone()),
            Err(e) => {
                error!("Failed to get circulating supply: {}", e);
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
