use std::time::Duration;

use async_trait::async_trait;
use log::{error, info, trace};

use crate::ephemera::reward::{EpochOperations, RewardManager, V2};
use crate::epoch_operations::MixnodeWithPerformance;

pub(crate) mod aggregator;

#[async_trait]
impl EpochOperations for RewardManager<V2> {
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
