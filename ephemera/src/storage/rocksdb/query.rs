use std::sync::Arc;

use log::trace;
use rocksdb::TransactionDB;

use crate::block::types::block::Block;
use crate::network::PeerId;
use crate::storage::rocksdb::{
    block_hash_key, block_height_key, certificates_key, last_block_key, members_key,
    merkle_tree_key,
};
use crate::utilities::crypto::Certificate;
use crate::utilities::merkle::MerkleTree;

pub struct Database {
    database: Arc<TransactionDB>,
}

impl Database {
    #[allow(dead_code)]
    pub fn new(db: Arc<TransactionDB>) -> Database {
        Database { database: db }
    }

    pub(crate) fn get_block_by_hash(&self, block_hash: &str) -> anyhow::Result<Option<Block>> {
        trace!("Getting block by id: {:?}", block_hash);

        let block_hash_key = block_hash_key(block_hash);

        let block = if let Some(block) = self.database.get(block_hash_key)? {
            let block = serde_json::from_slice::<Block>(&block)?;
            trace!("Found block: {}", block.header);
            Some(block)
        } else {
            trace!("Didn't find block");
            None
        };
        Ok(block)
    }

    pub(crate) fn get_last_block(&self) -> anyhow::Result<Option<Block>> {
        trace!("Getting last block");

        if let Some(block_hash) = self.database.get(last_block_key())? {
            let block_hash = String::from_utf8(block_hash)?;
            self.get_block_by_hash(&block_hash)
        } else {
            trace!("Unable to get last block");
            Ok(None)
        }
    }

    pub(crate) fn get_block_by_height(&self, height: u64) -> anyhow::Result<Option<Block>> {
        trace!("Getting block by height: {}", height);

        if let Some(block_hash) = self.database.get(block_height_key(height))? {
            let block_hash = String::from_utf8(block_hash)?;
            self.get_block_by_hash(&block_hash)
        } else {
            trace!("Didn't find block");
            Ok(None)
        }
    }

    pub(crate) fn get_block_certificates(
        &self,
        block_hash: &str,
    ) -> anyhow::Result<Option<Vec<Certificate>>> {
        trace!("Getting block signatures: {}", block_hash);

        let certificates_key = certificates_key(block_hash);

        if let Some(certificates) = self.database.get(certificates_key)? {
            let certificates: Vec<Certificate> = serde_json::from_slice(&certificates)?;
            trace!("Found certificates: {:?}", certificates);
            Ok(Some(certificates))
        } else {
            trace!("Didn't find signatures");
            Ok(None)
        }
    }

    pub(crate) fn get_block_broadcast_group(
        &self,
        block_hash: &str,
    ) -> anyhow::Result<Option<Vec<PeerId>>> {
        trace!("Getting block broadcast group: {}", block_hash);

        let members_key = members_key(block_hash);

        if let Some(members) = self.database.get(members_key)? {
            let members: Vec<PeerId> = serde_json::from_slice(&members)?;
            trace!("Found members: {:?}", members);
            Ok(Some(members))
        } else {
            trace!("Didn't find members");
            Ok(None)
        }
    }

    pub(crate) fn get_block_merkle_tree(
        &self,
        block_hash: &str,
    ) -> anyhow::Result<Option<MerkleTree>> {
        trace!("Getting block merkle tree: {}", block_hash);

        let merkle_tree_key = merkle_tree_key(block_hash);

        if let Some(merkle_tree) = self.database.get(merkle_tree_key)? {
            let merkle_tree: MerkleTree = serde_json::from_slice(&merkle_tree)?;
            trace!("Found merkle tree: {:?}", merkle_tree);
            Ok(Some(merkle_tree))
        } else {
            trace!("Didn't find merkle tree");
            Ok(None)
        }
    }
}
