use ephemera::{
    configuration::Configuration,
    ephemera_api::{
        ApiBlock, ApiEphemeraMessage, Application, ApplicationResult, CheckBlockResult,
        RemoveMessages,
    },
};
use log::{debug, error, info};

use crate::ephemera::contract::MixnodeToReward;
use crate::ephemera::peers::NymApiEphemeraPeerInfo;

pub(crate) struct RewardsEphemeraApplicationConfig {
    /// Percentage of messages relative to total number of peers
    pub(crate) peers_rewards_threshold: u64,
}

pub(crate) struct RewardsEphemeraApplication {
    peer_info: NymApiEphemeraPeerInfo,
    app_config: RewardsEphemeraApplicationConfig,
}

impl RewardsEphemeraApplication {
    pub(crate) fn init(ephemera_config: Configuration) -> anyhow::Result<Self> {
        let peer_info =
            match NymApiEphemeraPeerInfo::from_ephemera_dev_cluster_conf(&ephemera_config) {
                Ok(info) => info,
                Err(err) => {
                    error!("Failed to load peers info: {}", err);
                    return Err(err);
                }
            };
        let app_config = RewardsEphemeraApplicationConfig {
            peers_rewards_threshold: peer_info.get_peers_count() as u64,
        };
        Ok(Self {
            peer_info,
            app_config,
        })
    }
}

/// - TODO: We should also check that the messages has expected label(like epoch 100)
///         because next block should have only reward info for correct epoch.
impl Application for RewardsEphemeraApplication {
    /// Perform validation checks:
    /// - Check that the transaction has a valid signature, we don't want to accept garbage messages
    ///   or messages from unknown peers
    fn check_tx(&self, tx: ApiEphemeraMessage) -> ApplicationResult<bool> {
        if serde_json::from_slice::<Vec<MixnodeToReward>>(&tx.data).is_err() {
            error!("Message is not a valid Reward message");
            return Ok(false);
        }
        Ok(true)

        //TODO
        //PS! message label should also be part of the message hash to prevent replay attacks
    }

    /// Agree to accept the block if it contains threshold number of transactions
    /// We trust that transactions are valid(checked by check_tx)
    fn check_block(&self, block: &ApiBlock) -> ApplicationResult<CheckBlockResult> {
        info!("Block message count: {}", block.message_count());

        let block_threshold = ((block.message_count() as f64
            / self.peer_info.get_peers_count() as f64)
            * 100.0) as u64;

        if block_threshold > 100 {
            error!("Block threshold is greater than 100%!. We expected only single message from each peer");
            return Ok(CheckBlockResult::RejectAndRemoveMessages(
                RemoveMessages::All,
            ));
        }

        if block_threshold >= self.app_config.peers_rewards_threshold {
            info!(
                "Block accepted {}:{}",
                block.header.height, block.header.hash
            );
            Ok(CheckBlockResult::Accept)
        } else {
            debug!("Block rejected: not enough messages");
            Ok(CheckBlockResult::Reject)
        }
    }

    /// It is possible to use this method as a callback to get notified when block is committed
    fn deliver_block(&self, _block: ApiBlock) -> ApplicationResult<()> {
        Ok(())
    }
}
