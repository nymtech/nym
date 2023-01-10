// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::CirculatingSupplyCache;
use crate::support::nyxd::Client;
use anyhow::Result;
use std::sync::atomic::Ordering;
use std::time::Duration;
use task::TaskClient;
use tokio::time;
use validator_client::nyxd::Coin;

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

    async fn refresh(&self) -> Result<()> {
        let _ = &self.nyxd_client;
        let mixmining_reserve = Coin::new(0, "unym");
        let vesting_tokens = Coin::new(0, "unym");
        let circulating_supply = Coin::new(0, "unym");

        // let mixmining_temp_account = "n1299fhjdafamwc2gha723nkkewvu56u5xn78t9j"
        //     .parse::<AccountId>()
        //     .unwrap();
        //
        // let mixmining_temp = self
        //     .nyxd_client
        //     .get_balance(mixmining_temp_account)
        //     .await?
        //     .unwrap();
        //
        // let mixmining_contract_account = MIXNET_CONTRACT_ADDRESS.parse::<AccountId>().unwrap();
        //
        // let mixmining_contract = self
        //     .nyxd_client
        //     .get_balance(mixmining_contract_account)
        //     .await?
        //     .unwrap();
        //
        // let vesting_contract_account = VESTING_CONTRACT_ADDRESS.parse::<AccountId>().unwrap();
        //
        // let vesting_contract = self
        //     .nyxd_client
        //     .get_balance(vesting_contract_account)
        //     .await?
        //     .unwrap();
        //
        // let team_account = "n10fe8degw253uhezlfwaw555tw846u78waa8tc6"
        //     .parse::<AccountId>()
        //     .unwrap();
        //
        // let team = self.nyxd_client.get_balance(team_account).await?.unwrap();
        //
        // let company_account1 = "n104glfyskvrnx9u4upgqnz67axma72m5we3qaj4"
        //     .parse::<AccountId>()
        //     .unwrap();
        //
        // let company1 = self
        //     .nyxd_client
        //     .get_balance(company_account1)
        //     .await?
        //     .unwrap();
        //
        // let company_account2 = "n1yuagfmwvwyjn0g4q6vx8was35kc7tqner7lyq8"
        //     .parse::<AccountId>()
        //     .unwrap();
        //
        // let company2 = self
        //     .nyxd_client
        //     .get_balance(company_account2)
        //     .await?
        //     .unwrap();
        //
        // let investors_account = "n1rp46vs4kddfjufx38cl6etyxtcqpjfhg5mmqey"
        //     .parse::<AccountId>()
        //     .unwrap();
        //
        // let investors = self
        //     .nyxd_client
        //     .get_balance(investors_account)
        //     .await?
        //     .unwrap();
        //
        // let circulating_supply = Coin::new(
        //     1_000_000_000_000_000
        //         - mixmining_temp.amount
        //         - mixmining_contract.amount
        //         - vesting_contract.amount
        //         - team.amount
        //         - company1.amount
        //         - company2.amount
        //         - investors.amount,
        //     "unym", //TODO: this should be a constant
        // );
        //
        // log::info!(
        //     "Updating circulating supply cache. Circulating supply is now: {}",
        //     circulating_supply
        // );

        self.cache
            .update(mixmining_reserve, vesting_tokens, circulating_supply)
            .await;
        Ok(())
    }
}
