// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::data::CirculatingSupplyCacheData;
use cosmwasm_std::Addr;
use nym_api_requests::models::CirculatingSupplyResponse;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::Coin;
use std::ops::Deref;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{error, info};

mod data;
pub(crate) mod refresher;

#[derive(Debug, Error)]
enum CirculatingSupplyCacheError {
    // this can only happen if somebody decides to set their staking address
    // before https://github.com/nymtech/nym/pull/2796 is deployed
    #[error("vesting account owned by {owner} with id {account_id} appeared more than once in the query response")]
    DuplicateVestingAccountEntry { owner: Addr, account_id: u32 },

    // this can happen if somehow the query was incomplete, like some paged sub-query didn't return full result
    // or there's a bug with paging. or if, somehow, a vesting account got removed from the contract
    #[error("got an inconsistent number of vesting account. received data on {got}, but expected {expected}")]
    InconsistentNumberOfVestingAccounts { expected: usize, got: usize },

    #[error(transparent)]
    ClientError {
        #[from]
        source: NyxdError,
    },
}

/// A cache for the circulating supply of the network. Circulating supply is calculated by
/// taking the initial supply of 1bn coins, and subtracting the amount of coins that are
/// in the mixmining pool and tied up in vesting.
///
/// The cache is quite simple and does not include an update listener that the other caches have.
#[derive(Clone)]
pub(crate) struct CirculatingSupplyCache {
    initialised: Arc<AtomicBool>,
    data: Arc<RwLock<CirculatingSupplyCacheData>>,
}

impl CirculatingSupplyCache {
    pub(crate) fn new(mix_denom: String) -> CirculatingSupplyCache {
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

    pub(crate) async fn update(&self, mixmining_reserve: Coin, vesting_tokens: Coin) {
        let mut cache = self.data.write().await;

        let mut circulating_supply = cache.total_supply.clone();
        circulating_supply.amount -= mixmining_reserve.amount;
        circulating_supply.amount -= vesting_tokens.amount;

        info!("Updating circulating supply cache");
        info!("the mixmining reserve is now {mixmining_reserve}");
        info!("the number of tokens still vesting is now {vesting_tokens}");
        info!("the circulating supply is now {circulating_supply}");

        cache.mixmining_reserve.unchecked_update(mixmining_reserve);
        cache.vesting_tokens.unchecked_update(vesting_tokens);
        cache
            .circulating_supply
            .unchecked_update(circulating_supply);
    }
}
