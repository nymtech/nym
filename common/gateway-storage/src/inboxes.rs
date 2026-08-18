// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::StoredMessage;
use time::OffsetDateTime;
use tracing::debug;

#[derive(Clone)]
pub struct InboxManager {
    connection_pool: sqlx::SqlitePool,
    /// Maximum number of messages that can be obtained from the database per operation.
    /// It is used to prevent out of memory errors in the case of client receiving a lot of data while
    /// offline and then loading it all at once when he comes back online.
    retrieval_limit: i64,
}

impl InboxManager {
    /// Creates new instance of the `InboxManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool, mut retrieval_limit: i64) -> Self {
        // TODO: make this into a hard error instead
        if retrieval_limit == 0 {
            retrieval_limit = 100;
        }

        InboxManager {
            connection_pool,
            retrieval_limit,
        }
    }

    /// Inserts new message to the storage for an offline client for future retrieval.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    /// * `content`: raw content of the message to store.
    ///
    /// Store a message for a client that has registered with this gateway at some point.
    ///
    /// Returns `false`, having inserted nothing, when no `shared_keys` entry exists for the
    /// address. Retrieval requires the shared key established at registration, so a message for
    /// an address that never registered here can never be collected; and since the recipient is
    /// chosen freely by whoever sent the packet, inserting it anyway lets an unauthenticated
    /// sender grow this table without bound (up to the stale-message cutoff).
    pub(crate) async fn insert_message(
        &self,
        client_address_bs58: &str,
        content: Vec<u8>,
    ) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query!(
            r#"
                INSERT INTO message_store(client_address_bs58, content)
                SELECT ?, ?
                WHERE EXISTS(SELECT 1 FROM shared_keys WHERE client_address_bs58 = ?)
            "#,
            client_address_bs58,
            content,
            client_address_bs58,
        )
        .execute(&self.connection_pool)
        .await?
        .rows_affected();

        Ok(inserted > 0)
    }

    /// Retrieves messages stored for the particular client specified by the provided address.
    ///
    /// It also respects the specified retrieval limit. If there are more messages stored than allowed
    /// by the limit, it returns id of the last message retrieved to indicate start of the next query.
    ///
    /// # Arguments
    ///
    /// * `client_address_bs58`: base58-encoded address of the client
    /// * `start_after`: optional starting id of the messages to grab
    ///
    /// returns the retrieved messages alongside optional id of the last message retrieved if
    /// there are more messages to retrieve.
    pub(crate) async fn get_messages(
        &self,
        client_address_bs58: &str,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), sqlx::Error> {
        // get 1 additional message to check whether there will be more to grab
        // next time
        let limit = self.retrieval_limit + 1;
        let start_after = start_after.unwrap_or(-1);

        let mut res: Vec<StoredMessage> = sqlx::query_as(
            r#"
                SELECT id, client_address_bs58, content, timestamp
                FROM message_store
                WHERE client_address_bs58 = ? AND id > ?
                ORDER BY id ASC
                LIMIT ?;
            "#,
        )
        .bind(client_address_bs58)
        .bind(start_after)
        .bind(limit)
        .fetch_all(&self.connection_pool)
        .await?;

        if res.len() > self.retrieval_limit as usize {
            res.truncate(self.retrieval_limit as usize);
            // given retrieval_limit > 0, unwrap will not fail
            #[allow(clippy::unwrap_used)]
            let start_after = res.last().unwrap().id;
            Ok((res, Some(start_after)))
            //
        } else {
            Ok((res, None))
        }
    }

    /// Removes message with the specified id
    ///
    /// # Arguments
    ///
    /// * `id`: id of the message to remove
    pub(crate) async fn remove_message(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM message_store WHERE id = ?", id)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn remove_messages_for_client(
        &self,
        client_address_bs58: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM message_store WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    #[cfg(test)]
    async fn message_count(&self, client_address_bs58: &str) -> i64 {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM message_store WHERE client_address_bs58 = ?",
            client_address_bs58
        )
        .fetch_one(&self.connection_pool)
        .await
        .unwrap()
    }

    pub async fn remove_stale(&self, cutoff: OffsetDateTime) -> Result<(), sqlx::Error> {
        let affected = sqlx::query!("DELETE FROM message_store WHERE timestamp < ?", cutoff)
            .execute(&self.connection_pool)
            .await?
            .rows_affected();
        debug!("Removed {affected} stale messages");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const REGISTERED: &str = "registered-client-address";
    const NEVER_REGISTERED: &str = "never-registered-client-address";

    async fn setup() -> InboxManager {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory SQLite pool");
        let migrations_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        sqlx::migrate::Migrator::new(migrations_path.as_path())
            .await
            .expect("failed to find migrations")
            .run(&pool)
            .await
            .expect("failed to run migrations");

        // stand in for a completed registration handshake
        sqlx::query!("INSERT INTO clients(id, client_type) VALUES (1, 'entry_mixnet')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!(
            "INSERT INTO shared_keys(client_id, client_address_bs58, derived_aes256_gcm_siv_key) VALUES (1, ?, x'00')",
            REGISTERED
        )
        .execute(&pool)
        .await
        .unwrap();

        InboxManager::new(pool, 100)
    }

    #[tokio::test]
    async fn message_for_a_registered_client_is_stored() {
        let manager = setup().await;

        assert!(
            manager
                .insert_message(REGISTERED, vec![1, 2, 3])
                .await
                .unwrap()
        );
        assert_eq!(1, manager.message_count(REGISTERED).await);
    }

    #[tokio::test]
    async fn message_for_a_client_that_never_registered_is_not_stored() {
        let manager = setup().await;

        assert!(
            !manager
                .insert_message(NEVER_REGISTERED, vec![1, 2, 3])
                .await
                .unwrap()
        );
        assert_eq!(0, manager.message_count(NEVER_REGISTERED).await);
    }

    #[tokio::test]
    async fn a_flood_for_unregistered_recipients_stores_nothing() {
        let manager = setup().await;

        for i in 0..64 {
            let recipient = format!("{NEVER_REGISTERED}-{i}");
            assert!(
                !manager
                    .insert_message(&recipient, vec![0; 64])
                    .await
                    .unwrap()
            );
        }

        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM message_store")
            .fetch_one(&manager.connection_pool)
            .await
            .unwrap();
        assert_eq!(0, total);
    }
}
