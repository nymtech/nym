// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::CirculatingSupplyCache;
use crate::support::nyxd::Client;
use cosmwasm_std::coin;
use nym_contracts_common::truncate_decimal;
use nym_task::TaskClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::Coin;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time;
use tracing::{error, trace};

pub(crate) struct CirculatingSupplyCacheRefresher {
    nyxd_client: Client,
    cache: CirculatingSupplyCache,
    caching_interval: Duration,
}

impl CirculatingSupplyCacheRefresher {
    pub(crate) fn new(
        nyxd_client: Client,
        cache: CirculatingSupplyCache,
        caching_interval: Duration,
    ) -> Self {
        CirculatingSupplyCacheRefresher {
            nyxd_client,
            cache,
            caching_interval,
        }
    }

    pub(crate) async fn run(&self, mut shutdown: TaskClient) {
        let mut interval = time::interval(self.caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    tokio::select! {
                        biased;
                        _ = shutdown.recv() => {
                            trace!("CirculatingSupplyCacheRefresher: Received shutdown");
                        }
                        ret = self.refresh() => {
                            if let Err(err) = ret {
                                error!("Failed to refresh circulating supply cache - {err}");
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
                    trace!("CirculatingSupplyCacheRefresher: Received shutdown");
                }
            }
        }
    }

    async fn get_mixmining_reserve(&self, mix_denom: &str) -> Result<Coin, NyxdError> {
        let reward_pool = self
            .nyxd_client
            .get_current_rewarding_parameters()
            .await?
            .interval
            .reward_pool;

        Ok(Coin::new(truncate_decimal(reward_pool).u128(), mix_denom))
    }

    async fn get_total_vesting_tokens(&self, mix_denom: &str) -> Result<Coin, NyxdError> {
        Ok(coin(0, mix_denom).into())
    }

    async fn refresh(&self) -> Result<(), NyxdError> {
        let chain_details = self.nyxd_client.chain_details().await;
        let mix_denom = &chain_details.mix_denom.base;

        let mixmining_reserve = self.get_mixmining_reserve(mix_denom).await?;
        let vesting_tokens = self.get_total_vesting_tokens(mix_denom).await?;

        self.cache.update(mixmining_reserve, vesting_tokens).await;
        Ok(())
    }
}
