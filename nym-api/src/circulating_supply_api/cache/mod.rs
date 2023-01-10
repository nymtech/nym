// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::data::CirculatingSupplyCacheData;
use nym_api_requests::models::CirculatingSupplyResponse;
use rocket::fairing::AdHoc;
use std::ops::Deref;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::RwLock;
use tokio::time;
use validator_client::nyxd::Coin;

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
    fn new(mix_denom: String) -> CirculatingSupplyCache {
        CirculatingSupplyCache {
            initialised: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(CirculatingSupplyCacheData::new(mix_denom))),
        }
    }

    pub(crate) async fn get_circulating_supply(&self) -> Option<CirculatingSupplyResponse> {
        match time::timeout(Duration::from_millis(100), self.data.read()).await {
            Ok(cache) => Some(cache.deref().into()),
            Err(err) => {
                error!("Failed to get circulating supply: {err}");
                None
            }
        }
    }

    pub(crate) fn stage(mix_denom: String) -> AdHoc {
        AdHoc::on_ignite("Circulating Supply Cache Stage", |rocket| async {
            rocket.manage(Self::new(mix_denom))
        })
    }

    pub(crate) async fn update(
        &self,
        mixmining_reserve: Coin,
        vesting_tokens: Coin,
        circulating_supply: Coin,
    ) {
        let mut cache = self.data.write().await;
        cache.mixmining_reserve.update(mixmining_reserve);
        cache.vesting_tokens.update(vesting_tokens);
        cache.circulating_supply.update(circulating_supply);
    }
}
