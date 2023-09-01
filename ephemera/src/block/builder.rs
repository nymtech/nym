use std::collections::HashSet;
use std::{sync::Arc, time::Duration};

use log::{debug, info};

use crate::block::manager::State;
use crate::peer::ToPeerId;
use crate::{
    block::{
        manager::{BlockChainState, BlockManager},
        message_pool::MessagePool,
        producer::BlockProducer,
        types::block::Block,
    },
    broadcast::signing::BlockSigner,
    config::BlockManagerConfiguration,
    crypto::Keypair,
    storage::EphemeraDatabase,
};

pub(crate) struct BlockManagerBuilder {
    config: BlockManagerConfiguration,
    block_producer: BlockProducer,
    keypair: Arc<Keypair>,
}

impl BlockManagerBuilder {
    pub(crate) fn new(config: BlockManagerConfiguration, keypair: Arc<Keypair>) -> Self {
        let block_producer = BlockProducer::new(keypair.peer_id());
        Self {
            config,
            block_producer,
            keypair,
        }
    }

    pub(crate) fn build<D: EphemeraDatabase + ?Sized>(
        self,
        storage: &mut D,
    ) -> anyhow::Result<BlockManager> {
        let mut most_recent_block = storage.get_last_block()?;
        if most_recent_block.is_none() {
            //Although Ephemera is not a blockchain(chain of historically dependent blocks),
            //it's helpful to have some sort of notion of progress in time. So we use the concept of height.
            //The genesis block helps to define the start of it.

            info!("No last block found in database. Creating genesis block.");

            let genesis_block = Block::new_genesis_block(self.block_producer.peer_id);
            storage.store_block(&genesis_block, HashSet::new(), HashSet::new())?;
            most_recent_block = Some(genesis_block);
        }

        let last_created_block = most_recent_block.expect("Block should be present");
        debug!("Most recent block: {:?}", last_created_block);

        let block_signer = BlockSigner::new(self.keypair.clone());
        let message_pool = MessagePool::new();
        let block_chain_state = BlockChainState::new(last_created_block);
        let block_creation_interval =
            tokio::time::interval(Duration::from_secs(self.config.creation_interval_sec));

        Ok(BlockManager {
            config: self.config,
            block_producer: self.block_producer,
            block_signer,
            message_pool,
            block_chain_state,
            state: State::Paused,
            backoff: None,
            block_creation_interval,
        })
    }
}
