use async_trait::async_trait;
use log::{debug, info};

use crate::ephemera::reward::{EpochOperations, RewardManager, V1};
use crate::epoch_operations::MixnodeWithPerformance;

#[async_trait]
impl EpochOperations for RewardManager<V1> {
    async fn perform_epoch_operations(&mut self) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        let start = self.epoch.current_epoch_start_time().timestamp() as u64;
        let end = self.epoch.current_epoch_end_time().timestamp() as u64;
        info!("Calculating rewards for interval {} - {}", start, end);

        let rewards = self.calculate_rewards_for_previous_epoch().await?;
        let nr_of_rewards = rewards.len();
        debug!("Calculated rewards: {:?}", rewards);

        let mut storage = self.storage.lock().await;
        storage.save_rewarding_results(self.epoch.current_epoch_numer(), nr_of_rewards)?;

        Ok(rewards)
    }
}
