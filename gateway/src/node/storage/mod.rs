// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::connection_handler::coconut::PendingCredential;
use crate::node::storage::bandwidth::BandwidthManager;
use crate::node::storage::credential::CredentialManager;
use crate::node::storage::error::StorageError;
use crate::node::storage::inboxes::InboxManager;
use crate::node::storage::models::{PersistedSharedKeys, StoredMessage};
use crate::node::storage::shared_keys::SharedKeysManager;
use async_trait::async_trait;
use log::{debug, error};
use nym_compact_ecash::scheme::EcashCredential;
use nym_compact_ecash::Base58;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::DestinationAddressBytes;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::NymApiClient;
use sqlx::ConnectOptions;
use std::path::Path;
use std::str::FromStr;
use url::Url;

mod bandwidth;
mod credential;
pub(crate) mod error;
mod inboxes;
mod models;
mod shared_keys;

#[async_trait]
pub(crate) trait Storage: Send + Sync {
    /// Inserts provided derived shared keys into the database.
    /// If keys previously existed for the provided client, they are overwritten with the new data.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `shared_keys`: shared encryption (AES128CTR) and mac (hmac-blake3) derived shared keys to store.
    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &SharedKeys,
    ) -> Result<(), StorageError>;

    /// Tries to retrieve shared keys stored for the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    async fn get_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<PersistedSharedKeys>, StorageError>;

    /// Removes from the database shared keys derived with the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    // currently there is no code flow that causes removal (not overwriting)
    // of the stored keys. However, retain the function for consistency and completion sake
    #[allow(dead_code)]
    async fn remove_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError>;

    /// Inserts new message to the storage for an offline client for future retrieval.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `message`: raw message to store.
    async fn store_message(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), StorageError>;

    /// Retrieves messages stored for the particular client specified by the provided address.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `start_after`: optional starting id of the messages to grab
    ///
    /// returns the retrieved messages alongside optional id of the last message retrieved if
    /// there are more messages to retrieve.
    async fn retrieve_messages(
        &self,
        client_address: DestinationAddressBytes,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), StorageError>;

    /// Removes messages with the specified ids
    ///
    /// # Arguments
    ///
    /// * `ids`: ids of the messages to remove
    async fn remove_messages(&self, ids: Vec<i64>) -> Result<(), StorageError>;

    /// Creates a new bandwidth entry for the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    async fn create_bandwidth_entry(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError>;

    /// Tries to retrieve available bandwidth for the particular client.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    async fn get_available_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<i64>, StorageError>;

    /// Increases available bandwidth of the particular client by the specified amount.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `amount`: amount of available bandwidth to be added to the client.
    async fn increase_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
        amount: i64,
    ) -> Result<(), StorageError>;

    /// Decreases available bandwidth of the particular client by the specified amount.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `amount`: amount of available bandwidth to be removed from the client.
    async fn consume_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
        amount: i64,
    ) -> Result<(), StorageError>;

    /// Stored the accepted credential
    ///
    /// # Arguments
    ///
    /// * `credential`: credential to store
    async fn insert_credential(&self, credential: EcashCredential) -> Result<(), StorageError>;

    /// Store a pending credential
    ///
    /// # Arguments
    ///
    /// * `pending`: pending credential to store
    async fn insert_pending_credential(
        &self,
        pending: PendingCredential,
    ) -> Result<(), StorageError>;

    /// Remove a pending credential
    ///
    /// # Arguments
    ///
    /// * `id`: id of the pending credential to remove
    async fn remove_pending_credential(&self, id: i64) -> Result<(), StorageError>;

    /// Get all pending credentials
    ///
    async fn get_all_pending_credential(
        &self,
    ) -> Result<Vec<(i64, PendingCredential)>, StorageError>;
}

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct PersistentStorage {
    shared_key_manager: SharedKeysManager,
    inbox_manager: InboxManager,
    bandwidth_manager: BandwidthManager,
    credential_manager: CredentialManager,
}

impl PersistentStorage {
    /// Initialises `PersistentStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    /// * `message_retrieval_limit`: maximum number of stored client messages that can be retrieved at once.
    pub async fn init<P: AsRef<Path> + Send>(
        database_path: P,
        message_retrieval_limit: i64,
    ) -> Result<Self, StorageError> {
        debug!(
            "Attempting to connect to database {:?}",
            database_path.as_ref().as_os_str()
        );

        // TODO: we can inject here more stuff based on our gateway global config
        // struct. Maybe different pool size or timeout intervals?
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to perform migration on the SQLx database: {err}");
            return Err(err.into());
        }

        // the cloning here are cheap as connection pool is stored behind an Arc
        Ok(PersistentStorage {
            shared_key_manager: SharedKeysManager::new(connection_pool.clone()),
            inbox_manager: InboxManager::new(connection_pool.clone(), message_retrieval_limit),
            bandwidth_manager: BandwidthManager::new(connection_pool.clone()),
            credential_manager: CredentialManager::new(connection_pool),
        })
    }
}

#[async_trait]
impl Storage for PersistentStorage {
    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &SharedKeys,
    ) -> Result<(), StorageError> {
        let persisted_shared_keys = PersistedSharedKeys {
            client_address_bs58: client_address.as_base58_string(),
            derived_aes128_ctr_blake3_hmac_keys_bs58: shared_keys.to_base58_string(),
        };
        self.shared_key_manager
            .insert_shared_keys(persisted_shared_keys)
            .await?;
        Ok(())
    }

    async fn get_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<PersistedSharedKeys>, StorageError> {
        let keys = self
            .shared_key_manager
            .get_shared_keys(&client_address.as_base58_string())
            .await?;
        Ok(keys)
    }

    #[allow(dead_code)]
    async fn remove_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        self.shared_key_manager
            .remove_shared_keys(&client_address.as_base58_string())
            .await?;
        Ok(())
    }

    async fn store_message(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inbox_manager
            .insert_message(&client_address.as_base58_string(), message)
            .await?;
        Ok(())
    }

    async fn retrieve_messages(
        &self,
        client_address: DestinationAddressBytes,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), StorageError> {
        let messages = self
            .inbox_manager
            .get_messages(&client_address.as_base58_string(), start_after)
            .await?;
        Ok(messages)
    }

    async fn remove_messages(&self, ids: Vec<i64>) -> Result<(), StorageError> {
        for id in ids {
            self.inbox_manager.remove_message(id).await?;
        }
        Ok(())
    }

    async fn create_bandwidth_entry(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        self.bandwidth_manager
            .insert_new_client(&client_address.as_base58_string())
            .await?;
        Ok(())
    }

    async fn get_available_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<i64>, StorageError> {
        let res = self
            .bandwidth_manager
            .get_available_bandwidth(&client_address.as_base58_string())
            .await
            .map(|bandwidth_option| bandwidth_option.map(|bandwidth| bandwidth.available))?;
        Ok(res)
    }

    async fn increase_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
        amount: i64,
    ) -> Result<(), StorageError> {
        self.bandwidth_manager
            .increase_available_bandwidth(&client_address.as_base58_string(), amount)
            .await?;
        Ok(())
    }

    async fn consume_bandwidth(
        &self,
        client_address: DestinationAddressBytes,
        amount: i64,
    ) -> Result<(), StorageError> {
        self.bandwidth_manager
            .decrease_available_bandwidth(&client_address.as_base58_string(), amount)
            .await?;
        Ok(())
    }

    async fn insert_credential(&self, credential: EcashCredential) -> Result<(), StorageError> {
        self.credential_manager
            .insert_credential(credential.to_bs58())
            .await?;
        Ok(())
    }

    async fn insert_pending_credential(
        &self,
        pending: PendingCredential,
    ) -> Result<(), StorageError> {
        self.credential_manager
            .insert_pending_credential(
                pending.credential.to_bs58(),
                pending.address.into(),
                pending.client.api_url().to_string(),
            )
            .await?;
        Ok(())
    }

    async fn remove_pending_credential(&self, id: i64) -> Result<(), StorageError> {
        self.credential_manager
            .remove_pending_credential(id)
            .await?;
        Ok(())
    }

    async fn get_all_pending_credential(
        &self,
    ) -> Result<Vec<(i64, PendingCredential)>, StorageError> {
        let credentials: Vec<_> = self
            .credential_manager
            .get_all_pending_credential()
            .await?
            .into_iter()
            .map(|stored_pending| {
                let credential = EcashCredential::try_from_bs58(stored_pending.credential)
                    .map_err(|err| StorageError::DataCorruptionError(err.to_string()))?;
                Ok((
                    stored_pending.id,
                    PendingCredential {
                        credential,
                        address: AccountId::from_str(&stored_pending.address)
                            .map_err(|err| StorageError::DataCorruptionError(err.to_string()))?,
                        client: NymApiClient::new(
                            Url::from_str(&stored_pending.api_url).map_err(|err| {
                                StorageError::DataCorruptionError(err.to_string())
                            })?,
                        ),
                    },
                ))
            })
            .collect();
        credentials.into_iter().collect()
    }
}

/// In-memory implementation of `Storage`. The intention is primarily in testing environments.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct InMemStorage;

//#[cfg(test)]
//impl InMemStorage {
//    #[allow(unused)]
//    async fn init<P: AsRef<Path> + Send>() -> Result<Self, StorageError> {
//        todo!()
//    }
//}

#[cfg(test)]
#[async_trait]
impl Storage for InMemStorage {
    async fn insert_shared_keys(
        &self,
        _client_address: DestinationAddressBytes,
        _shared_keys: &SharedKeys,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn get_shared_keys(
        &self,
        _client_address: DestinationAddressBytes,
    ) -> Result<Option<PersistedSharedKeys>, StorageError> {
        todo!()
    }

    async fn remove_shared_keys(
        &self,
        _client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn store_message(
        &self,
        _client_address: DestinationAddressBytes,
        _message: Vec<u8>,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn retrieve_messages(
        &self,
        _client_address: DestinationAddressBytes,
        _start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), StorageError> {
        todo!()
    }

    async fn remove_messages(&self, _ids: Vec<i64>) -> Result<(), StorageError> {
        todo!()
    }

    async fn create_bandwidth_entry(
        &self,
        _client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn get_available_bandwidth(
        &self,
        _client_address: DestinationAddressBytes,
    ) -> Result<Option<i64>, StorageError> {
        todo!()
    }

    async fn increase_bandwidth(
        &self,
        _client_address: DestinationAddressBytes,
        _amount: i64,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn consume_bandwidth(
        &self,
        _client_address: DestinationAddressBytes,
        _amount: i64,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn insert_credential(&self, _credential: EcashCredential) -> Result<(), StorageError> {
        todo!()
    }

    async fn insert_pending_credential(
        &self,
        _pending: PendingCredential,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn remove_pending_credential(&self, _id: i64) -> Result<(), StorageError> {
        todo!()
    }

    async fn get_all_pending_credential(
        &self,
    ) -> Result<Vec<(i64, PendingCredential)>, StorageError> {
        todo!()
    }
}
