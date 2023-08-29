use std::collections::HashSet;
use std::sync::Arc;

use log::info;
use rocksdb::{TransactionDB, TransactionDBOptions};

use crate::block::types::block::Block;
use crate::config::DatabaseConfiguration;
use crate::peer::PeerId;
use crate::storage::rocksdb::query::Database;
use crate::storage::rocksdb::store::DbStore;
use crate::storage::EphemeraDatabase;
use crate::storage::Result;
use crate::utilities::crypto::Certificate;
use crate::utilities::merkle::MerkleTree;

pub(crate) mod query;
pub(crate) mod store;

pub(crate) struct RocksDbStorage {
    pub(crate) db_store: DbStore,
    pub(crate) db_query: Database,
}

const PREFIX_LAST_BLOCK_KEY: &str = "last_block";
const PREFIX_BLOCK_HASH: &str = "block_hash";
const PREFIX_BLOCK_HEIGHT: &str = "block_height";
const PREFIX_CERTIFICATES: &str = "block_certificates";
const PREFIX_MEMBERS: &str = "block_members";
const MERKLE_TREE: &str = "merkle_tree";

impl RocksDbStorage {
    pub fn open(db_conf: &DatabaseConfiguration) -> Result<Self> {
        info!("Opening RocksDB database at {}", db_conf.rocksdb_path);

        let mut options = rocksdb::Options::default();
        options.create_if_missing(db_conf.create_if_not_exists);

        let db = TransactionDB::open(
            &options,
            &TransactionDBOptions::default(),
            db_conf.rocksdb_path.clone(),
        )
        .map_err(|err| anyhow::anyhow!(err))?;

        let db = Arc::new(db);
        let db_store = DbStore::new(db.clone());
        let db_query = Database::new(db);
        let storage = Self { db_store, db_query };

        info!("Opened RocksDB database at {}", db_conf.rocksdb_path);
        Ok(storage)
    }
}

impl EphemeraDatabase for RocksDbStorage {
    fn get_block_by_hash(&self, block_id: &str) -> Result<Option<Block>> {
        self.db_query
            .get_block_by_hash(block_id)
            .map_err(Into::into)
    }

    fn get_last_block(&self) -> Result<Option<Block>> {
        self.db_query.get_last_block().map_err(Into::into)
    }

    fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        self.db_query
            .get_block_by_height(height)
            .map_err(Into::into)
    }

    fn get_block_certificates(&self, block_id: &str) -> Result<Option<Vec<Certificate>>> {
        self.db_query
            .get_block_certificates(block_id)
            .map_err(Into::into)
    }

    fn get_block_broadcast_group(&self, block_id: &str) -> Result<Option<Vec<PeerId>>> {
        self.db_query
            .get_block_broadcast_group(block_id)
            .map_err(Into::into)
    }

    fn store_block(
        &mut self,
        block: &Block,
        certificates: HashSet<Certificate>,
        members: HashSet<PeerId>,
    ) -> Result<()> {
        self.db_store
            .store_block(block, certificates, members)
            .map_err(Into::into)
    }

    fn get_block_merkle_tree(&self, block_hash: &str) -> Result<Option<MerkleTree>> {
        self.db_query
            .get_block_merkle_tree(block_hash)
            .map_err(Into::into)
    }
}

fn block_hash_key(block_hash: &str) -> String {
    format!("{PREFIX_BLOCK_HASH}:{block_hash}")
}

fn block_height_key(height: u64) -> String {
    format!("{PREFIX_BLOCK_HEIGHT}:{height}")
}

fn last_block_key() -> String {
    PREFIX_LAST_BLOCK_KEY.to_string()
}

fn certificates_key(block_hash: &str) -> String {
    format!("{PREFIX_CERTIFICATES}:{block_hash}",)
}

fn members_key(block_hash: &str) -> String {
    format!("{PREFIX_MEMBERS}:{block_hash}",)
}

fn merkle_tree_key(block_hash: &str) -> String {
    format!("{MERKLE_TREE}:{block_hash}",)
}
