// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use log::{debug, info, trace};
use std::time::Duration;

use crate::epoch_operations::MixnodeWithPerformance;
use ephemera::{
    crypto::Keypair,
    ephemera_api::{self, ApiBlock, ApiEphemeraMessage, ApiError, CommandExecutor},
};

use super::epoch::Epoch;
use super::reward::aggregator::RewardsAggregator;
use super::Args;

pub(crate) mod aggregator;

pub struct EphemeraAccess {
    pub(crate) api: CommandExecutor,
    pub(crate) key_pair: Keypair,
}

#[async_trait]
impl EpochOperations for RewardManager {
    async fn perform_epoch_operations(
        &mut self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        //Submit our own rewards message.
        //It will be included in the next block(ours and/or others)
        self.send_rewards_to_ephemera(rewards).await?;

        //Assuming that next block includes our rewards message
        //This assumptions need to be "configured" by the application.
        let prev_block = self.get_last_block().await?;
        let next_height = prev_block.header.height + 1;

        //Poll next block which should include all messages from the previous epoch from almost all Nym-Api nodes
        let mut counter = 0;
        info!(
            "Waiting for block with height {next_height} maximum {} seconds",
            self.args.block_polling_max_attempts * self.args.block_polling_interval_seconds
        );
        loop {
            if counter > self.args.block_polling_max_attempts {
                error!("Block for height {next_height} is not available after {counter} attempts");
                break;
            }
            tokio::select! {
                Ok(Some(block)) = self.get_block_by_height(next_height) => {
                    info!("Received local block with height {next_height}, hash:{:?}", block.header.hash);
                    if let Ok(agg_rewards) = self.try_aggregate_rewards(block.clone()).await{
                        info!("Submitted rewards to smart contract");
                        let epoch_id = self.epoch.current_epoch_numer();
                        self.store_in_dht(epoch_id, &block).await?;
                        info!("Stored rewards in DHT");
                        return Ok(agg_rewards);
                    }
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(self.args.block_polling_interval_seconds)) => {
                    trace!("Block for height {next_height} is not available yet, waiting...");
                }
            }
            counter += 1;
        }

        info!("Querying for block with height {next_height} from the DHT");
        counter = 0;
        let epoch_id = self.epoch.current_epoch_numer();
        loop {
            if counter > self.args.block_polling_max_attempts {
                error!(
                    "DHT: Block for height {next_height} is not available after {counter} attempts"
                );
                break;
            }
            tokio::select! {
               Ok(Some(block)) = self.query_dht(epoch_id) => {
                   info!("DHT: Received block {block}");
                   break;
               }
               _= tokio::time::sleep(Duration::from_secs(self.args.block_polling_interval_seconds)) => {
                   trace!("DHT: Block for height {next_height} is not available in yet, waiting...");
               }
            }
            counter += 1;
        }

        // TODO: query smart contract for the nym-api which was able to submit rewards and query its block.
        // TODO: Because each Ephemera "sees" all blocks during RB then it might be safe to save them locally
        // TODO: already during RB. In case of failure of that node.

        info!("Finished reward calculation for previous epoch");
        Ok(vec![])
    }
}

impl EphemeraAccess {
    pub(crate) fn new(api: CommandExecutor, key_pair: Keypair) -> Self {
        Self { api, key_pair }
    }
}

#[async_trait]
pub(crate) trait EpochOperations {
    async fn perform_epoch_operations(
        &mut self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>>;
}

pub(crate) struct RewardManager {
    pub epoch: Epoch,
    pub args: Args,
    pub ephemera_access: Option<EphemeraAccess>,
    aggregator: Option<RewardsAggregator>,
}

impl RewardManager
where
    Self: EpochOperations,
{
    pub(crate) fn new(
        args: Args,
        ephemera_access: Option<EphemeraAccess>,
        aggregator: Option<RewardsAggregator>,
        epoch: Epoch,
    ) -> Self {
        info!(
            "Starting RewardManager with epoch nr {}",
            epoch.current_epoch_numer()
        );
        Self {
            epoch,
            args,
            ephemera_access,
            aggregator,
        }
    }

    pub(crate) async fn get_last_block(&self) -> Result<ApiBlock, ApiError> {
        let access = self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set");
        let block = access.api.get_last_block().await?;
        Ok(block)
    }

    pub(crate) async fn get_block_by_height(
        &self,
        height: u64,
    ) -> Result<Option<ApiBlock>, ApiError> {
        let access = self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set");
        let block = access.api.get_block_by_height(height).await?;
        Ok(block)
    }

    pub(crate) async fn store_in_dht(&self, epoch_id: u64, block: &ApiBlock) -> anyhow::Result<()> {
        info!("Storing ourselves as 'winner' in DHT for epoch id: {epoch_id:?}");

        let access = self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set");

        let key = format!("epoch_id_{epoch_id}").into_bytes();
        let value = serde_json::to_vec(&block).expect("Failed to serialize block");

        access.api.store_in_dht(key, value).await?;
        info!("Sent store request to DHT");
        Ok(())
    }

    pub(crate) async fn query_dht(&self, epoch_id: u64) -> anyhow::Result<Option<ApiBlock>> {
        let access = self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set");

        let key = format!("epoch_id_{epoch_id}").into_bytes();

        match access.api.query_dht(key).await? {
            None => {
                info!("No 'winner' found for epoch id from DHT: {epoch_id:?}");
                Ok(None)
            }
            Some((_, block)) => {
                let block = serde_json::from_slice(block.as_slice())?;
                info!("'Winner' found for epoch id from DHT: {epoch_id:?} - {block:?}");
                Ok(Some(block))
            }
        }
    }

    pub(crate) async fn send_rewards_to_ephemera(
        &self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<()> {
        let ephemera_msg = self.create_ephemera_message(rewards)?;
        debug!("Sending rewards to ephemera: {:?}", ephemera_msg);

        let access = self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set");

        access.api.send_ephemera_message(ephemera_msg).await?;
        Ok(())
    }

    fn create_ephemera_message(
        &self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<ApiEphemeraMessage> {
        let keypair = &self
            .ephemera_access
            .as_ref()
            .expect("Ephemera access not set")
            .key_pair;

        let label = self.epoch.current_epoch_numer().to_string();
        let data = serde_json::to_vec(&rewards)?;
        let raw_message = ephemera_api::RawApiEphemeraMessage::new(label, data);

        let certificate = ephemera_api::ApiCertificate::prepare(keypair, &raw_message)?;
        let signed_message = ApiEphemeraMessage::new(raw_message, certificate);

        Ok(signed_message)
    }

    //By current assumption, all nodes will try submit their aggregated rewards
    //and contract will reject all but first one.
    async fn try_aggregate_rewards(
        &mut self,
        block: ApiBlock,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        info!(
            "Calculating aggregated rewards from block with height: {:?}",
            block.header.height
        );
        let mut mix_node_rewards = vec![];

        for message in block.messages {
            trace!("Message: {}", message);
            let mix_node_reward: Vec<MixnodeWithPerformance> =
                serde_json::from_slice(&message.data)?;
            mix_node_rewards.push(mix_node_reward);
        }

        let aggregated_rewards = self.aggregator().aggregate(mix_node_rewards)?;
        debug!("Aggregated rewards: {:?}", aggregated_rewards);

        Ok(aggregated_rewards)
    }

    fn aggregator(&self) -> &RewardsAggregator {
        self.aggregator.as_ref().expect("Aggregator not set")
    }
}
