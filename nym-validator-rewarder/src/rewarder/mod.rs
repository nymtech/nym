// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use crate::rewarder::block_signing::EpochSigning;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::helpers::consensus_address_to_account;
use nym_network_defaults::NymNetworkDetails;
use nym_task::TaskManager;
use nym_validator_client::nyxd::{AccountId, Coin, StakingQueryClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use nyxd_scraper::{models, NyxdScraper};
use sha2::{Digest, Sha256};
use std::cmp::min;
use std::collections::HashMap;
use std::ops::{Add, Range};
use std::str::FromStr;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::{interval_at, Instant};
use tracing::{debug, error, info, instrument};

mod block_signing;
mod epoch;
mod helpers;
mod tasks;

pub struct Rewarder {
    current_epoch: Epoch,
    rpc_client: QueryHttpRpcNyxdClient,

    config: Config,
    nyxd_scraper: NyxdScraper,
}

impl Rewarder {
    pub async fn new(config: Config) -> Result<Self, NymRewarderError> {
        let nyxd_scraper = NyxdScraper::new(config.scraper_config()).await?;
        let rpc_client = QueryHttpRpcNyxdClient::connect(
            config.rpc_client_config(),
            config.base.upstream_nyxd.as_str(),
        )?;

        Ok(Rewarder {
            current_epoch: Epoch::first()?,
            rpc_client,
            config,
            nyxd_scraper,
        })
    }

    async fn get_voting_power(
        &self,
        address: &str,
        height_range: Range<i64>,
    ) -> Result<Option<i64>, NymRewarderError> {
        for height in height_range {
            if let Some(precommit) = self
                .nyxd_scraper
                .storage
                .get_precommit(address, height)
                .await?
            {
                return Ok(Some(precommit.voting_power));
            }
        }

        Ok(None)
    }

    async fn get_signed_blocks(&self) -> Result<EpochSigning, NymRewarderError> {
        info!(
            "looking up block signers for epoch {} ({} - {})",
            self.current_epoch.id,
            self.current_epoch.start_rfc3339(),
            self.current_epoch.end_rfc3339()
        );

        let validators = self.nyxd_scraper.storage.get_all_known_validators().await?;
        let epoch_start = self.current_epoch.start;
        let epoch_end = self.current_epoch.end;
        let first_block = self
            .nyxd_scraper
            .storage
            .get_first_block_height_after(epoch_start)
            .await?
            .unwrap_or_default();
        let last_block = self
            .nyxd_scraper
            .storage
            .get_last_block_height_before(epoch_end)
            .await?
            .unwrap_or_default();

        // each validator MUST be online at some point during the first 20 blocks, otherwise they're not getting anything.
        let vp_range_end = min(first_block + 20, last_block);
        let vp_range = first_block..vp_range_end;

        let mut total_vp = 0;
        let mut signed_in_epoch = HashMap::new();
        for validator in validators {
            let Some(vp) = self
                .get_voting_power(&validator.consensus_address, vp_range.clone())
                .await?
            else {
                continue;
            };
            total_vp += vp;

            let signed = self
                .nyxd_scraper
                .storage
                .get_signed_between_times(&validator.consensus_address, epoch_start, epoch_end)
                .await?;
            signed_in_epoch.insert(validator, (signed, vp));
        }

        let total = self
            .nyxd_scraper
            .storage
            .get_blocks_between(epoch_start, epoch_end)
            .await?;

        Ok(EpochSigning::construct(total, total_vp, signed_in_epoch))
    }

    #[instrument(skip(self,budget), fields(budget = %budget))]

    async fn calculate_block_signing_rewards(
        &mut self,
        budget: Coin,
    ) -> Result<HashMap<models::Validator, Coin>, NymRewarderError> {
        info!("calculating reward shares");
        let signed = self.get_signed_blocks().await?;

        debug!("details: {signed:?}");

        Ok(signed.rewarding_amounts(&budget))
    }

    #[instrument(skip(self,budget), fields(budget = %budget))]
    async fn calculate_credential_rewards(
        &mut self,
        budget: Coin,
    ) -> Result<HashMap<models::Validator, Coin>, NymRewarderError> {
        info!("calculating reward shares");
        Ok(HashMap::new())
    }

    async fn determine_epoch_rewards(
        &mut self,
    ) -> Result<HashMap<models::Validator, (AccountId, Coin)>, NymRewarderError> {
        let epoch_budget = &self.config.rewarding.epoch_budget;
        let denom = &epoch_budget.denom;
        let signing_budget = Coin::new(
            (self.config.rewarding.ratios.block_signing * epoch_budget.amount as f64) as u128,
            denom,
        );
        let credential_budget = Coin::new(
            (self.config.rewarding.ratios.credential_issuance * epoch_budget.amount as f64) as u128,
            denom,
        );

        let signing_rewards = self.calculate_block_signing_rewards(signing_budget).await?;
        let credential_rewards = self.calculate_credential_rewards(credential_budget).await?;

        let mut rewards: HashMap<models::Validator, (AccountId, Coin)> = HashMap::new();
        let validators2 = self
            .rpc_client
            .validators("".to_string(), None)
            .await
            .unwrap();

        let mut monikers = HashMap::new();
        for v in validators2.validators {
            let val = v.consensus_pubkey.unwrap();
            // println!("{:?}", val);

            let digest = Sha256::digest(&val.to_bytes()).to_vec();

            // println!("{}", String::from_utf8_lossy(&val));
            // assert_eq!(val.len(), 32);

            let consensus_key = AccountId::new("nvalcons", &digest[..20])
                .unwrap()
                .to_string();
            let moniker = v.description.unwrap().moniker;
            let acc = v.operator_address;
            let acc = AccountId::new("n", &acc.to_bytes()).unwrap();
            monikers.insert(consensus_key, (moniker, acc));
        }

        for (val, amount) in &signing_rewards {
            // let oper = AccountId::new("nvaloper", &acc.to_bytes()).unwrap();
            // let moniker = get_moniker(acc.clone()).await;

            let (moniker, acc) = monikers.get(&val.consensus_address).unwrap();
            println!("{moniker}: {acc} signing: {amount} credentials: XXX")
            //
        }

        // for (validator, amount) in signing_rewards {
        //     let account = consensus_address_to_account(&validator)?;
        //     rewards.insert(validator, (account, amount));
        // }
        //
        // for (validator, amount) in credential_rewards {
        //     let account = consensus_address_to_account(&validator)?;
        //
        //     if let Some(existing) = rewards.get_mut(&validator) {
        //         assert_eq!(existing.0, account);
        //         existing.1.amount += amount.amount;
        //     } else {
        //         rewards.insert(validator, (account, amount));
        //     }
        // }

        Ok(rewards)
    }

    async fn send_rewards(
        &self,
        amounts: HashMap<AccountId, Coin>,
    ) -> Result<(), NymRewarderError> {
        Ok(())
    }

    async fn handle_epoch_end(&mut self) {
        info!("handling the epoch end");

        let rewards = match self.determine_epoch_rewards().await {
            Ok(rewards) => rewards,
            Err(err) => {
                error!("failed to determine epoch rewards: {err}");
                return;
            }
        };

        // for (_, (acc, amount)) in rewards {
        //     // let oper = AccountId::new("nvaloper", &acc.to_bytes()).unwrap();
        //     // let moniker = get_moniker(acc.clone()).await;
        //
        //     let moniker = validators2.validators.iter().find(|v| {
        //         v.consensus_pubkey
        //
        //     })
        //
        //     println!("{moniker}: {acc} signing: {amount} credentials: XXX")
        //     //
        // }

        /*

        let budget = Coin::new(667_000_000, "unym");
        let rewards = foo.rewarding_amounts(&budget);

        println!("{rewards:#?}");
        666_378_383
        let bar: u128 = rewards.into_values().map(|v| v.amount).sum();
        println!("summed: {bar}");

         */
        self.current_epoch = self.current_epoch.next();
    }

    pub async fn run(mut self) -> Result<(), NymRewarderError> {
        info!("Starting nym validators rewarder");

        // setup shutdowns
        let mut task_manager = TaskManager::new(5);

        self.nyxd_scraper.start().await?;
        //
        // tokio::time::sleep(Duration::from_secs(3000)).await;

        // rewarding epochs last from :00 to :00
        self.current_epoch.end = OffsetDateTime::now_utc();
        self.current_epoch.start = self.current_epoch.end - Duration::from_secs(60 * 60);

        println!("sleepiing for 10");
        tokio::time::sleep(Duration::from_secs(10)).await;

        let until_end = self.current_epoch.until_end();

        info!(
            "the first epoch will finish in {} secs",
            until_end.as_secs()
        );
        let mut epoch_ticker = interval_at(Instant::now().add(until_end), Epoch::LENGTH);

        loop {
            tokio::select! {
                biased;
                interrupt_res = task_manager.catch_interrupt() => {
                    info!("received interrupt");
                    if let Err(err) = interrupt_res {
                        error!("runtime interrupt failure: {err}")
                    }
                    break;
                }
                _ = epoch_ticker.tick() => self.handle_epoch_end().await
            }
        }

        self.nyxd_scraper.stop().await;

        /*
           task 1:
           on timer:
               - go to DKG contract
               - get all coconut signers
               - for each of them get the info, verify, etc

           task 2:
           on timer (or maybe per block?):
               - query abci endpoint for VP
               - also maybe missed blocks, etc

        */

        todo!()
    }
}

fn val_to_nym(addr: &str) -> String {
    let foo: AccountId = addr.parse().unwrap();
    let bar = AccountId::new("n", &foo.to_bytes()).unwrap();
    bar.to_string()
}

fn make_url(oper: &str) -> String {
    format!("https://rpc.nymtech.net/api/cosmos/staking/v1beta1/validators/{oper}")
}

async fn get_moniker(addr: AccountId) -> String {
    let oper = AccountId::new("nvaloper", &addr.to_bytes())
        .unwrap()
        .to_string();

    let foo: serde_json::Value = reqwest::get(make_url(&oper))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    println!("raw: {foo:?}");

    let Some(a) = foo.as_object() else {
        return "UNKNOWN".to_string();
    };
    let Some(b) = a.get("validator").and_then(|o| o.as_object()) else {
        return "UNKNOWN".to_string();
    };
    let Some(c) = b.get("description").and_then(|o| o.as_object()) else {
        return "UNKNOWN".to_string();
    };
    let moniker = c.get("moniker").unwrap().as_str().unwrap();
    moniker.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn aaa() {
        let addr = AccountId::from_str("nvalcons1yn8kzqna703x5a6wh449ylw70u5drjejx5t6dz").unwrap();
        println!("{}", AccountId::new("n", &addr.to_bytes()).unwrap());
        println!("{}", AccountId::new("nvaloper", &addr.to_bytes()).unwrap());
        println!("{}", AccountId::new("nvalcons", &addr.to_bytes()).unwrap());
        // println!("{}", AccountId::new("n", &addr.to_bytes()).unwrap());

        // let b = val_to_nym("nvaloper1q8cnx8s06q7ralnskqvj0acvqgacau6djqkm3z");
        // println!("{b}");
    }

    #[tokio::test]
    async fn bar() {
        let oper = "nvaloper18xr68spwm96vvehuvwf6ay9er0gd7q7ae8w8ns";
        let foo: serde_json::Value = reqwest::get(make_url(oper))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let a = foo.as_object().unwrap();
        let b = a.get("validator").unwrap().as_object().unwrap();
        let c = b.get("description").unwrap().as_object().unwrap();
        let moniker = c.get("moniker").unwrap().as_str().unwrap();
        println!("moniker: {moniker}")
    }
}
