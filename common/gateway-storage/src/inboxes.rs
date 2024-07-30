// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::StoredMessage;

#[derive(Clone)]
pub(crate) struct InboxManager {
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
    pub(crate) async fn insert_message(
        &self,
        client_address_bs58: &str,
        content: Vec<u8>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO message_store(client_address_bs58, content) VALUES (?, ?)",
            client_address_bs58,
            content,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
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
        let mut res = if let Some(start_after) = start_after {
            sqlx::query_as!(
                StoredMessage,
                r#"
                    SELECT 
                        id as "id!",
                        client_address_bs58 as "client_address_bs58!",
                        content as "content!" 
                    FROM message_store 
                    WHERE client_address_bs58 = ? AND id > ?
                    ORDER BY id ASC
                    LIMIT ?;
                "#,
                client_address_bs58,
                start_after,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        } else {
            sqlx::query_as!(
                StoredMessage,
                r#"
                   SELECT 
                        id as "id!",
                        client_address_bs58 as "client_address_bs58!",
                        content as "content!"
                    FROM message_store
                    WHERE client_address_bs58 = ?
                    ORDER BY id ASC
                    LIMIT ?;
                "#,
                client_address_bs58,
                limit
            )
            .fetch_all(&self.connection_pool)
            .await?
        };

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
}
