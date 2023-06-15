use async_trait::async_trait;

use crate::ephemera::reward::{EpochOperations, RewardManager, V1};
use crate::epoch_operations::MixnodeWithPerformance;

#[async_trait]
impl EpochOperations for RewardManager<V1> {
    async fn perform_epoch_operations(
        &mut self,
        rewards: Vec<MixnodeWithPerformance>,
    ) -> anyhow::Result<Vec<MixnodeWithPerformance>> {
        Ok(rewards)
    }
}
