//! # Mixnet contract simulation
//!
//! # Overview
//! Instead of been an actual decentralised contract in blockchain, it's a http server and a database.
//! Besides that there's not really any difference(also minus fees and accounts).
//!
//! It allows to submit rewards.
//! It also manages epoch.

use std::sync::Arc;
use std::thread::sleep;

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use chrono::{DateTime, Duration, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use ephemera::configuration::Configuration;

use super::contract::http::{get_epoch, get_nym_apis, submit_reward};
use super::epoch::{Epoch, EpochInfo};
use super::peers::NymApiEphemeraPeerInfo;
use super::storage::db::{ContractStorageType, Storage};
use super::ContractArgs;

pub mod http;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MixnodeToReward {
    pub mix_id: usize,
    pub performance: u8,
}

impl MixnodeToReward {
    pub fn new(mix_id: usize, performance: u8) -> Self {
        Self {
            mix_id,
            performance,
        }
    }
}

//Simulates smart contract functionality related to rewarding and members management.
pub struct SmartContract {
    pub(crate) storage: Storage<ContractStorageType>,
    pub(crate) epoch: Epoch,
    pub(crate) peer_info: NymApiEphemeraPeerInfo,
}

impl SmartContract {
    pub fn new(
        storage: Storage<ContractStorageType>,
        epoch: Epoch,
        ephemera_config: Configuration,
    ) -> Self {
        let peer_info =
            NymApiEphemeraPeerInfo::from_ephemera_dev_cluster_conf(&ephemera_config).unwrap();
        Self {
            storage,
            epoch,
            peer_info,
        }
    }

    pub async fn start(args: ContractArgs, ephemera_config: Configuration) {
        info!("Starting smart contract");

        let mut storage: Storage<ContractStorageType> = Storage::init();

        let epoch = Self::get_epoch(args.epoch_duration_seconds, &mut storage);
        info!("Epoch info: {epoch}");

        let smart_contract = SmartContract::new(storage, epoch, ephemera_config);
        let smart_contract = Arc::new(Mutex::new(smart_contract));

        let smart_contract_http = smart_contract.clone();
        let mut server = HttpServer::new(move || {
            App::new()
                .app_data(Data::new(smart_contract_http.clone()))
                .service(submit_reward)
                .service(get_epoch)
                .service(get_nym_apis)
        })
        .bind(args.url)
        .unwrap()
        .run();

        info!("Smart contract started!");

        let mut epoch_save_interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            tokio::select! {
                _ = &mut server => {
                    info!("Smart contract stopped!");
                }
                _ = epoch_save_interval.tick() => {
                    smart_contract.lock().await.update_epoch(args.epoch_duration_seconds);
                }
            }
        }
    }

    fn get_epoch(epoch_duration_seconds: u64, storage: &mut Storage<ContractStorageType>) -> Epoch {
        let epoch = storage.get_epoch().unwrap();
        match epoch {
            None => {
                info!("No epoch info found, creating new one");
                let epoch = EpochInfo::new(0, Utc::now().timestamp(), epoch_duration_seconds);
                storage.save_epoch(&epoch).unwrap();
                Epoch::new(epoch)
            }
            Some(info) => {
                info!("Found epoch info, starting from it");
                Epoch::new(info)
            }
        }
    }

    fn update_epoch(&mut self, epoch_duration_seconds: u64) {
        let epoch_id = self.epoch.current_epoch_numer();
        let start_time = self.epoch.start_time.timestamp();

        let epoch = EpochInfo::new(epoch_id, start_time, epoch_duration_seconds);
        self.storage.update_epoch(&epoch).unwrap();
    }

    pub async fn submit_mix_rewards(
        &mut self,
        nym_api_id: &str,
        rewards: Vec<MixnodeToReward>,
    ) -> anyhow::Result<()> {
        let now: DateTime<Utc> = Utc::now();
        let epoch_id = self.epoch.current_epoch_numer();

        self.storage
            .contract_submit_mixnode_rewards(epoch_id, now.timestamp(), nym_api_id, rewards)
    }

    pub async fn get_epoch_from_db(&mut self) -> anyhow::Result<EpochInfo> {
        //Called by HTTP API
        //First time there's no epoch in database, so we need to wait for it to be
        //inserted by the contract at startup
        loop {
            let epoch = self.storage.get_epoch()?;
            if epoch.is_none() {
                sleep(Duration::seconds(1).to_std().unwrap());
                continue;
            }
            return Ok(epoch.unwrap());
        }
    }
}
