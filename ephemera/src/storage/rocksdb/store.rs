use std::collections::HashSet;
use std::sync::Arc;

use crate::block::types::block::Block;
use crate::network::PeerId;
use crate::storage::rocksdb::{
    block_hash_key, block_height_key, certificates_key, last_block_key, members_key,
    merkle_tree_key,
};
use log::{debug, trace};
use rocksdb::{TransactionDB, WriteBatchWithTransaction};

use crate::utilities::crypto::Certificate;

#[allow(clippy::module_name_repetitions)]
pub struct DbStore {
    connection: Arc<TransactionDB>,
}

impl DbStore {
    pub fn new(db: Arc<TransactionDB>) -> DbStore {
        DbStore { connection: db }
    }

    pub(crate) fn store_block(
        &self,
        block: &Block,
        certificates: HashSet<Certificate>,
        members: HashSet<PeerId>,
    ) -> anyhow::Result<()> {
        debug!("Storing block: {}", block.header);
        trace!("Storing block certificates: {}", certificates.len());

        let hash_str = block.header.hash.to_string();

        let block_id_key = block_hash_key(&hash_str);
        let certificates_key = certificates_key(&hash_str);
        let height_key = block_height_key(block.header.height);
        let members_key = members_key(&hash_str);
        let merkle_tree_key = merkle_tree_key(&hash_str);

        // Check UNIQUE constraints
        let existing_id = self.connection.get(&block_id_key)?;
        if existing_id.is_some() {
            return Err(anyhow::anyhow!("Block already exists"));
        }

        let mut batch = WriteBatchWithTransaction::<true>::default();

        //Store last block id(without prefix!)
        //May want to check that height is incremented by 1
        batch.put(last_block_key(), hash_str.clone());

        // Store block height
        batch.put(height_key.as_bytes(), hash_str);

        // Store block(without signature)
        let block_bytes = serde_json::to_vec::<Block>(block)?;
        batch.put(block_id_key.as_bytes(), block_bytes);

        // Store block certificates
        let certificates_bytes =
            serde_json::to_vec(&certificates.into_iter().collect::<Vec<Certificate>>())
                .map_err(|e| anyhow::anyhow!(e))?;
        batch.put(certificates_key.as_bytes(), certificates_bytes);

        // Store block members
        let members_bytes = serde_json::to_vec(&members.into_iter().collect::<Vec<PeerId>>())
            .map_err(|e| anyhow::anyhow!(e))?;
        batch.put(members_key.as_bytes(), members_bytes);

        //Store Merkle Tree
        let merkle_tree = block.merkle_tree()?;
        let merkle_tree_bytes = serde_json::to_vec(&merkle_tree).map_err(|e| anyhow::anyhow!(e))?;
        batch.put(merkle_tree_key.as_bytes(), merkle_tree_bytes);

        self.connection.write(batch)?;
        Ok(())
    }
}
