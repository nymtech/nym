use crate::block::types::block::Block;
use anyhow::Result;
use log::debug;
use rusqlite::{params, Connection, OpenFlags};
use std::collections::HashSet;

use crate::config::DatabaseConfiguration;
use crate::network::PeerId;
use crate::utilities::crypto::Certificate;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(db_conf: DatabaseConfiguration, flags: OpenFlags) -> Result<Database> {
        let connection = Connection::open_with_flags(db_conf.sqlite_path, flags)?;
        Ok(Database { connection })
    }

    pub(crate) fn store_block(
        &mut self,
        block: &Block,
        certificates: HashSet<Certificate>,
        members: HashSet<PeerId>,
    ) -> Result<()> {
        debug!("Storing block: {}", block.header);

        let hash = block.header.hash.to_string();
        let height = block.header.height;
        let block_bytes = serde_json::to_vec::<Block>(block).map_err(|e| anyhow::anyhow!(e))?;
        let certificates_bytes =
            serde_json::to_vec(&certificates.into_iter().collect::<Vec<Certificate>>())
                .map_err(|e| anyhow::anyhow!(e))?;
        let members_bytes = serde_json::to_vec(&members.into_iter().collect::<Vec<PeerId>>())
            .map_err(|e| anyhow::anyhow!(e))?;
        let merkle_tree = block.merkle_tree()?;
        let merkle_tree_bytes = serde_json::to_vec(&merkle_tree).map_err(|e| anyhow::anyhow!(e))?;

        let tx = self.connection.transaction()?;
        {
            let mut statement = tx.prepare_cached(
                "INSERT INTO blocks (block_hash, height, block) VALUES (?1, ?2, ?3)",
            )?;
            statement.execute(params![&hash, &height, &block_bytes,])?;

            let mut statement = tx.prepare_cached(
                "INSERT INTO block_certificates (block_hash, certificates) VALUES (?1, ?2)",
            )?;

            statement.execute(params![&hash, &certificates_bytes,])?;

            let mut statement = tx.prepare_cached(
                "INSERT INTO block_broadcast_group (block_hash, members) VALUES (?1, ?2)",
            )?;

            statement.execute(params![&hash, &members_bytes])?;

            //store Merkle Tree
            let mut statement = tx.prepare_cached(
                "INSERT INTO block_merkle_tree (block_hash, merkle_tree) VALUES (?1, ?2)",
            )?;

            statement.execute(params![&hash, &merkle_tree_bytes])?;
        }

        tx.commit()?;

        Ok(())
    }
}
