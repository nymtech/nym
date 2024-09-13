// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use bandwidth::BandwidthManager;
use clients::{ClientManager, ClientType};
use error::StorageError;
use inboxes::InboxManager;
use models::{
    Client, PersistedBandwidth, PersistedSharedKeys, RedemptionProposal, StoredMessage,
    VerifiedTicket, WireguardPeer,
};
use nym_credentials_interface::ClientTicket;
use nym_gateway_requests::registration::handshake::LegacySharedKeys;
use nym_sphinx::DestinationAddressBytes;
use shared_keys::SharedKeysManager;
use sqlx::ConnectOptions;
use std::path::Path;
use tickets::TicketStorageManager;
use time::OffsetDateTime;
use tracing::{debug, error};

pub mod bandwidth;
mod clients;
pub mod error;
mod inboxes;
pub mod models;
mod shared_keys;
mod tickets;
mod wireguard_peers;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_mixnet_client_id(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<i64, StorageError>;

    /// Inserts provided derived shared keys into the database.
    /// If keys previously existed for the provided client, they are overwritten with the new data.
    ///
    /// # Arguments
    ///
    /// * `client_address`: base58-encoded address of the client
    /// * `shared_keys`: shared encryption (AES128CTR) and mac (hmac-blake3) derived shared keys to store.
    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &LegacySharedKeys,
    ) -> Result<i64, StorageError>;

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

    /// Tries to retrieve a particular client.
    ///
    /// # Arguments
    ///
    /// * `client_id`: id of the client
    #[allow(dead_code)]
    async fn get_client(&self, client_id: i64) -> Result<Option<Client>, StorageError>;

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
    async fn create_bandwidth_entry(&self, client_id: i64) -> Result<(), StorageError>;

    /// Set the freepass expiration date of the particular client to the provided date.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    /// * `expiration`: the expiration date of the associated free pass.
    async fn set_expiration(
        &self,
        client_id: i64,
        expiration: OffsetDateTime,
    ) -> Result<(), StorageError>;

    /// Reset all the bandwidth
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client
    async fn reset_bandwidth(&self, client_id: i64) -> Result<(), StorageError>;

    /// Tries to retrieve available bandwidth for the particular client.
    async fn get_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<Option<PersistedBandwidth>, StorageError>;

    /// Increases specified client's bandwidth by the provided amount and returns the current value.
    async fn increase_bandwidth(&self, client_id: i64, amount: i64) -> Result<i64, StorageError>;

    async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        amount: i64,
    ) -> Result<(), StorageError>;

    #[allow(dead_code)]
    /// Decreases specified client's bandwidth by the provided amount and returns the current value.
    async fn decrease_bandwidth(&self, client_id: i64, amount: i64) -> Result<i64, StorageError>;

    async fn insert_epoch_signers(
        &self,
        epoch_id: i64,
        signer_ids: Vec<i64>,
    ) -> Result<(), StorageError>;

    async fn insert_received_ticket(
        &self,
        client_id: i64,
        received_at: OffsetDateTime,
        serial_number: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<i64, StorageError>;

    // note: this only checks very recent tickets that haven't yet been redeemed
    // (but it's better than nothing)
    /// Check if the ticket with the provided serial number if already present in the storage.
    ///
    /// # Arguments
    ///
    /// * `serial_number`: the unique serial number embedded in the ticket
    async fn contains_ticket(&self, serial_number: &[u8]) -> Result<bool, StorageError>;

    async fn insert_ticket_verification(
        &self,
        ticket_id: i64,
        signer_id: i64,
        verified_at: OffsetDateTime,
        accepted: bool,
    ) -> Result<(), StorageError>;

    async fn update_rejected_ticket(&self, ticket_id: i64) -> Result<(), StorageError>;

    async fn update_verified_ticket(&self, ticket_id: i64) -> Result<(), StorageError>;

    async fn remove_verified_ticket_binary_data(&self, ticket_id: i64) -> Result<(), StorageError>;

    async fn get_all_verified_tickets_with_sn(&self) -> Result<Vec<VerifiedTicket>, StorageError>;
    async fn get_all_proposed_tickets_with_sn(
        &self,
        proposal_id: u32,
    ) -> Result<Vec<VerifiedTicket>, StorageError>;

    async fn insert_redemption_proposal(
        &self,
        tickets: &[VerifiedTicket],
        proposal_id: u32,
        created_at: OffsetDateTime,
    ) -> Result<(), StorageError>;

    async fn clear_post_proposal_data(
        &self,
        proposal_id: u32,
        resolved_at: OffsetDateTime,
        rejected: bool,
    ) -> Result<(), StorageError>;

    async fn latest_proposal(&self) -> Result<Option<RedemptionProposal>, StorageError>;

    async fn get_all_unverified_tickets(&self) -> Result<Vec<ClientTicket>, StorageError>;
    async fn get_all_unresolved_proposals(&self) -> Result<Vec<i64>, StorageError>;
    async fn get_votes(&self, ticket_id: i64) -> Result<Vec<i64>, StorageError>;

    async fn get_signers(&self, epoch_id: i64) -> Result<Vec<i64>, StorageError>;

    /// Insert a wireguard peer in the storage.
    ///
    /// # Arguments
    ///
    /// * `peer`: wireguard peer data to be stored
    /// * `suspended`: if peer exists, but it's currently suspended
    async fn insert_wireguard_peer(
        &self,
        peer: &defguard_wireguard_rs::host::Peer,
        suspended: bool,
    ) -> Result<(), StorageError>;

    /// Tries to retrieve available bandwidth for the particular peer.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be retrieved.
    async fn get_wireguard_peer(
        &self,
        peer_public_key: &str,
    ) -> Result<Option<WireguardPeer>, StorageError>;

    /// Retrieves all wireguard peers.
    async fn get_all_wireguard_peers(&self) -> Result<Vec<WireguardPeer>, StorageError>;

    /// Remove a wireguard peer from the storage.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be removed.
    async fn remove_wireguard_peer(&self, peer_public_key: &str) -> Result<(), StorageError>;
}

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct PersistentStorage {
    client_manager: ClientManager,
    shared_key_manager: SharedKeysManager,
    inbox_manager: InboxManager,
    bandwidth_manager: BandwidthManager,
    ticket_manager: TicketStorageManager,
    wireguard_peer_manager: wireguard_peers::WgPeerManager,
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
            client_manager: clients::ClientManager::new(connection_pool.clone()),
            wireguard_peer_manager: wireguard_peers::WgPeerManager::new(connection_pool.clone()),
            shared_key_manager: SharedKeysManager::new(connection_pool.clone()),
            inbox_manager: InboxManager::new(connection_pool.clone(), message_retrieval_limit),
            bandwidth_manager: BandwidthManager::new(connection_pool.clone()),
            ticket_manager: TicketStorageManager::new(connection_pool),
        })
    }
}

#[async_trait]
impl Storage for PersistentStorage {
    async fn get_mixnet_client_id(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<i64, StorageError> {
        Ok(self
            .shared_key_manager
            .client_id(&client_address.as_base58_string())
            .await?)
    }

    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &LegacySharedKeys,
    ) -> Result<i64, StorageError> {
        let client_id = self
            .client_manager
            .insert_client(ClientType::EntryMixnet)
            .await?;
        self.shared_key_manager
            .insert_shared_keys(
                client_id,
                client_address.as_base58_string(),
                shared_keys.to_base58_string(),
            )
            .await?;
        Ok(client_id)
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

    async fn get_client(&self, client_id: i64) -> Result<Option<Client>, StorageError> {
        let client = self.client_manager.get_client(client_id).await?;
        Ok(client)
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

    async fn create_bandwidth_entry(&self, client_id: i64) -> Result<(), StorageError> {
        self.bandwidth_manager.insert_new_client(client_id).await?;
        Ok(())
    }

    async fn set_expiration(
        &self,
        client_id: i64,
        expiration: OffsetDateTime,
    ) -> Result<(), StorageError> {
        self.bandwidth_manager
            .set_expiration(client_id, expiration)
            .await?;
        Ok(())
    }

    async fn reset_bandwidth(&self, client_id: i64) -> Result<(), StorageError> {
        self.bandwidth_manager.reset_bandwidth(client_id).await?;
        Ok(())
    }

    async fn get_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<Option<PersistedBandwidth>, StorageError> {
        Ok(self
            .bandwidth_manager
            .get_available_bandwidth(client_id)
            .await?)
    }

    async fn increase_bandwidth(&self, client_id: i64, amount: i64) -> Result<i64, StorageError> {
        Ok(self
            .bandwidth_manager
            .increase_bandwidth(client_id, amount)
            .await?)
    }

    async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        amount: i64,
    ) -> Result<(), StorageError> {
        Ok(self
            .bandwidth_manager
            .revoke_ticket_bandwidth(ticket_id, amount)
            .await?)
    }

    async fn decrease_bandwidth(&self, client_id: i64, amount: i64) -> Result<i64, StorageError> {
        Ok(self
            .bandwidth_manager
            .decrease_bandwidth(client_id, amount)
            .await?)
    }

    async fn insert_epoch_signers(
        &self,
        epoch_id: i64,
        signer_ids: Vec<i64>,
    ) -> Result<(), StorageError> {
        self.ticket_manager
            .insert_ecash_signers(epoch_id, signer_ids)
            .await?;
        Ok(())
    }

    async fn insert_received_ticket(
        &self,
        client_id: i64,
        received_at: OffsetDateTime,
        serial_number: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<i64, StorageError> {
        // technically if we crash between those 2 calls we'll have a bit of data inconsistency,
        // but nothing too tragic. we just won't get paid for a single ticket
        let ticket_id = self
            .ticket_manager
            .insert_new_ticket(client_id, received_at)
            .await?;
        self.ticket_manager
            .insert_ticket_data(ticket_id, &serial_number, &data)
            .await?;

        Ok(ticket_id)
    }

    async fn contains_ticket(&self, serial_number: &[u8]) -> Result<bool, StorageError> {
        Ok(self.ticket_manager.has_ticket_data(serial_number).await?)
    }

    async fn insert_ticket_verification(
        &self,
        ticket_id: i64,
        signer_id: i64,
        verified_at: OffsetDateTime,
        accepted: bool,
    ) -> Result<(), StorageError> {
        self.ticket_manager
            .insert_ticket_verification(ticket_id, signer_id, verified_at, accepted)
            .await?;
        Ok(())
    }

    async fn update_rejected_ticket(&self, ticket_id: i64) -> Result<(), StorageError> {
        // set the ticket as rejected
        self.ticket_manager.set_rejected_ticket(ticket_id).await?;

        // drop all ticket_data - we no longer need it
        // TODO: or maybe we do as a proof of receiving bad data?
        self.ticket_manager.remove_ticket_data(ticket_id).await?;

        Ok(())
    }

    async fn update_verified_ticket(&self, ticket_id: i64) -> Result<(), StorageError> {
        // 1. insert into verified table
        self.ticket_manager
            .insert_verified_ticket(ticket_id)
            .await?;

        // TODO: maybe we want to leave that be until ticket gets fully redeemed instead?
        // 2. remove individual verifications
        self.ticket_manager
            .remove_ticket_verification(ticket_id)
            .await?;
        Ok(())
    }

    async fn remove_verified_ticket_binary_data(&self, ticket_id: i64) -> Result<(), StorageError> {
        self.ticket_manager
            .remove_binary_ticket_data(ticket_id)
            .await?;
        Ok(())
    }

    async fn get_all_verified_tickets_with_sn(&self) -> Result<Vec<VerifiedTicket>, StorageError> {
        Ok(self
            .ticket_manager
            .get_all_verified_tickets_with_sn()
            .await?)
    }

    async fn get_all_proposed_tickets_with_sn(
        &self,
        proposal_id: u32,
    ) -> Result<Vec<VerifiedTicket>, StorageError> {
        Ok(self
            .ticket_manager
            .get_all_proposed_tickets_with_sn(proposal_id as i64)
            .await?)
    }

    async fn insert_redemption_proposal(
        &self,
        tickets: &[VerifiedTicket],
        proposal_id: u32,
        created_at: OffsetDateTime,
    ) -> Result<(), StorageError> {
        // if we crash between those, there might a bit of an issue. we should revisit it later

        // 1. insert the actual proposal
        self.ticket_manager
            .insert_redemption_proposal(proposal_id as i64, created_at)
            .await?;

        // 2. update all the associated tickets
        self.ticket_manager
            .insert_verified_tickets_proposal_id(
                tickets.iter().map(|t| t.ticket_id),
                proposal_id as i64,
            )
            .await?;
        Ok(())
    }

    async fn clear_post_proposal_data(
        &self,
        proposal_id: u32,
        resolved_at: OffsetDateTime,
        rejected: bool,
    ) -> Result<(), StorageError> {
        // 1. update proposal metadata
        self.ticket_manager
            .update_redemption_proposal(proposal_id as i64, resolved_at, rejected)
            .await?;

        // 2. remove ticket data rows (we can drop serial numbers)
        self.ticket_manager
            .remove_redeemed_tickets_data(proposal_id as i64)
            .await?;

        // 3. remove verified tickets rows
        self.ticket_manager
            .remove_verified_tickets(proposal_id as i64)
            .await?;

        Ok(())
    }

    async fn latest_proposal(&self) -> Result<Option<RedemptionProposal>, StorageError> {
        Ok(self.ticket_manager.get_latest_redemption_proposal().await?)
    }

    async fn get_all_unverified_tickets(&self) -> Result<Vec<ClientTicket>, StorageError> {
        self.ticket_manager
            .get_unverified_tickets()
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    async fn get_all_unresolved_proposals(&self) -> Result<Vec<i64>, StorageError> {
        Ok(self
            .ticket_manager
            .get_all_unresolved_redemption_proposal_ids()
            .await?)
    }

    async fn get_votes(&self, ticket_id: i64) -> Result<Vec<i64>, StorageError> {
        Ok(self
            .ticket_manager
            .get_verification_votes(ticket_id)
            .await?)
    }

    async fn get_signers(&self, epoch_id: i64) -> Result<Vec<i64>, StorageError> {
        Ok(self.ticket_manager.get_epoch_signers(epoch_id).await?)
    }

    async fn insert_wireguard_peer(
        &self,
        peer: &defguard_wireguard_rs::host::Peer,
        suspended: bool,
    ) -> Result<(), StorageError> {
        let mut peer = WireguardPeer::from(peer.clone());
        peer.suspended = suspended;
        self.wireguard_peer_manager.insert_peer(&peer).await?;
        Ok(())
    }

    async fn get_wireguard_peer(
        &self,
        peer_public_key: &str,
    ) -> Result<Option<WireguardPeer>, StorageError> {
        let peer = self
            .wireguard_peer_manager
            .retrieve_peer(peer_public_key)
            .await?;
        Ok(peer)
    }

    async fn get_all_wireguard_peers(&self) -> Result<Vec<WireguardPeer>, StorageError> {
        let ret = self.wireguard_peer_manager.retrieve_all_peers().await?;
        Ok(ret)
    }

    async fn remove_wireguard_peer(&self, peer_public_key: &str) -> Result<(), StorageError> {
        self.wireguard_peer_manager
            .remove_peer(peer_public_key)
            .await?;
        Ok(())
    }
}
