use std::marker::PhantomData;

use async_trait::async_trait;
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};

use crate::epoch_operations::MixnodeWithPerformance;
use ephemera::{
    crypto::Keypair,
    ephemera_api::{self, ApiBlock, ApiEphemeraMessage, ApiError, CommandExecutor},
};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::MixId;

use super::epoch::Epoch;
use super::reward::new::aggregator::RewardsAggregator;
use super::Args;
use crate::support::nyxd;

pub(crate) mod new;
mod old;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MixnodeToReward {
    pub mix_id: MixId,
    pub performance: Performance,
}

pub(crate) struct V1;

pub(crate) struct V2;

pub struct EphemeraAccess {
    pub(crate) api: CommandExecutor,
    pub(crate) key_pair: Keypair,
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

pub(crate) struct RewardManager<V> {
    pub nyxd_client: nyxd::Client,

    pub epoch: Epoch,
    pub args: Args,
    pub version: PhantomData<V>,
    pub ephemera_access: Option<EphemeraAccess>,
    aggregator: Option<RewardsAggregator>,
}

impl<V> RewardManager<V>
where
    Self: EpochOperations,
{
    pub(crate) fn new(
        nyxd_client: nyxd::Client,
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
            nyxd_client,
            epoch,
            args,
            version: Default::default(),
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
            let mix_node_reward: Vec<MixnodeToReward> = serde_json::from_slice(&message.data)?;
            mix_node_rewards.push(mix_node_reward);
        }

        let aggregated_rewards = self.aggregator().aggregate(mix_node_rewards);
        debug!("Aggregated rewards: {:?}", aggregated_rewards);

        Ok(aggregated_rewards)
    }

    fn aggregator(&self) -> &RewardsAggregator {
        self.aggregator.as_ref().expect("Aggregator not set")
    }
}
