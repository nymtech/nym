use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info, trace};
use tokio::{sync::broadcast::Receiver, sync::Mutex};

use crate::epoch_operations::MixnodeWithPerformance;
use ephemera::{
    crypto::Keypair,
    ephemera_api::{self, ApiBlock, ApiEphemeraMessage, ApiError, CommandExecutor},
};
use nym_mixnet_contract_common::MixId;

use super::contract::MixnodeToReward;
use super::epoch::Epoch;
use super::reward::new::aggregator::RewardsAggregator;
use super::storage::db::{MetricsStorageType, Storage};
use super::{Args, NR_OF_MIX_NODES};
use crate::support::nyxd;

pub(crate) mod new;
mod old;

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
    async fn perform_epoch_operations(&mut self) -> anyhow::Result<Vec<MixnodeWithPerformance>>;
}

pub(crate) struct RewardManager<V> {
    pub storage: Arc<Mutex<Storage<MetricsStorageType>>>,
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
        storage: Arc<Mutex<Storage<MetricsStorageType>>>,
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
            storage,
            nyxd_client,
            epoch,
            args,
            version: Default::default(),
            ephemera_access,
            aggregator,
        }
    }

    pub(crate) async fn start(mut self, mut receiver: Receiver<()>) {
        loop {
            tokio::select! {
                _ = receiver.recv() => {
                    info!("Shutting down reward manager");
                    break;
                }
                _ =  self.epoch.wait_epoch_end() => {
                    info!("Rewarding epoch {} ...", self.epoch.current_epoch_numer());
                    if let Err(err) = self.perform_epoch_operations().await {
                        error!("Reward calculator failed: {}", err);
                        break;
                    }
                }
            }
        }
        info!("Reward manager stopped");
    }

    pub(crate) async fn calculate_rewards_for_previous_epoch(
        &self,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        let start = self.epoch.current_epoch_start_time().timestamp() as u64;
        let end = self.epoch.current_epoch_end_time().timestamp() as u64;
        info!("Calculating rewards for interval {} - {}", start, end);

        let mix_nodes = self.get_mix_nodes_to_reward();
        debug!("Mix nodes to reward: {:?}", mix_nodes);

        let storage = self.storage.lock().await;

        let mut uptimes = Vec::with_capacity(NR_OF_MIX_NODES as usize);
        for mix_id in mix_nodes {
            let reliability = storage.get_mixnode_average_reliability(mix_id, start, end)?;
            uptimes.push(MixnodeWithPerformance {
                mix_id,
                performance: reliability.unwrap_or_default(),
            });
        }

        Ok(uptimes)
    }

    pub(crate) async fn submit_rewards_to_contract(
        &self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<()> {
        info!("Submitting rewards to contract");
        let ret = self.nyxd_client.send_rewarding_messages(&rewards).await;

        if ret.is_err() {
            return Err(anyhow::anyhow!(
                "Failed to submit rewards to contract because of {:?}",
                ret
            ));
        }
        Ok(())
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

    fn get_mix_nodes_to_reward(&self) -> Vec<MixId> {
        (0..NR_OF_MIX_NODES).collect::<Vec<_>>()
    }
}
