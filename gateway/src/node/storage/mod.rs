// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::bandwidth::BandwidthManager;
use crate::node::storage::error::StorageError;
use crate::node::storage::inboxes::InboxManager;
use crate::node::storage::models::{PersistedSharedKeys, StoredMessage};
use crate::node::storage::shared_keys::SharedKeysManager;
use async_trait::async_trait;
use log::{debug, error};
use nym_credentials_interface::{Base58, BlindedSerialNumber};
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::DestinationAddressBytes;
use sqlx::ConnectOptions;
use std::path::Path;

mod bandwidth;
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

    /// Mark received credential as spent and insert it into the storage.
    ///
    /// # Arguments
    ///
    /// * `blinded_serial_number`: the unique blinded serial number embedded in the credential
    /// * `client_address`: address of the client that spent the credential
    async fn insert_spent_credential(
        &self,
        blinded_serial_number: BlindedSerialNumber,
        was_freepass: bool,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError>;

    /// Check if the credential with the provided blinded serial number if already present in the storage.
    ///
    /// # Arguments
    ///
    /// * `blinded_serial_number`: the unique blinded serial number embedded in the credential
    async fn contains_credential(
        &self,
        blinded_serial_number: &BlindedSerialNumber,
    ) -> Result<bool, StorageError>;
}

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct PersistentStorage {
    shared_key_manager: SharedKeysManager,
    inbox_manager: InboxManager,
    bandwidth_manager: BandwidthManager,
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
            bandwidth_manager: BandwidthManager::new(connection_pool),
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

    async fn insert_spent_credential(
        &self,
        blinded_serial_number: BlindedSerialNumber,
        was_freepass: bool,
        client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        self.bandwidth_manager
            .insert_spent_credential(
                &blinded_serial_number.to_bs58(),
                was_freepass,
                &client_address.as_base58_string(),
            )
            .await?;
        Ok(())
    }

    async fn contains_credential(
        &self,
        blinded_serial_number: &BlindedSerialNumber,
    ) -> Result<bool, StorageError> {
        let cred = self
            .bandwidth_manager
            .retrieve_spent_credential(&blinded_serial_number.to_bs58())
            .await?;

        Ok(cred.is_some())
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

    async fn insert_spent_credential(
        &self,
        _blinded_serial_number: BlindedSerialNumber,
        _was_freepass: bool,
        _client_address: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        todo!()
    }

    async fn contains_credential(
        &self,
        _blinded_serial_number: &BlindedSerialNumber,
    ) -> Result<bool, StorageError> {
        todo!()
    }
}
