use log::{error, info};
use rusqlite::Connection;
use std::collections::HashSet;

use crate::block::types::block::Block;
use crate::config::DatabaseConfiguration;
use crate::peer::PeerId;
use crate::storage::sqlite::query::DbQuery;
use crate::storage::sqlite::store::Database;
use crate::storage::EphemeraDatabase;
use crate::storage::Result;
use crate::utilities::crypto::Certificate;
use crate::utilities::merkle::MerkleTree;

pub(crate) mod query;
pub(crate) mod store;

mod migrations {
    use refinery::embed_migrations;

    embed_migrations!("migrations");
}

pub(crate) struct SqliteStorage {
    pub(crate) db_store: Database,
    pub(crate) db_query: DbQuery,
}

impl SqliteStorage {
    pub(crate) fn open(db_conf: DatabaseConfiguration) -> Result<Self> {
        let mut flags = rusqlite::OpenFlags::default();
        if !db_conf.create_if_not_exists {
            flags.remove(rusqlite::OpenFlags::SQLITE_OPEN_CREATE);
        }

        let mut connection = Connection::open_with_flags(db_conf.sqlite_path.clone(), flags)
            .map_err(|err| anyhow::anyhow!(err))?;
        Self::run_migrations(&mut connection)?;

        info!("Starting db backend with path: {}", db_conf.sqlite_path);
        let db_store = Database::open(db_conf.clone(), flags)?;
        let db_query = DbQuery::open(db_conf, flags)?;
        let storage = Self { db_store, db_query };
        Ok(storage)
    }

    pub(crate) fn run_migrations(connection: &mut Connection) -> Result<()> {
        info!("Running database migrations");
        match migrations::migrations::runner().run(connection) {
            Ok(ok) => {
                info!("Database migrations completed:{:?} ", ok);
                Ok(())
            }
            Err(err) => {
                error!("Database migrations failed: {}", err);
                Err(anyhow::anyhow!(err).into())
            }
        }
    }
}

impl EphemeraDatabase for SqliteStorage {
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
