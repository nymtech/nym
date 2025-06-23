// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use bandwidth::BandwidthManager;
use clients::{ClientManager, ClientType};
use models::{
    Client, PersistedBandwidth, PersistedSharedKeys, RedemptionProposal, StoredMessage,
    VerifiedTicket, WireguardPeer,
};
use nym_credentials_interface::ClientTicket;
use nym_gateway_requests::shared_key::SharedGatewayKey;
use nym_sphinx::DestinationAddressBytes;
use shared_keys::SharedKeysManager;
use sqlx::{
    sqlite::{SqliteAutoVacuum, SqliteSynchronous},
    ConnectOptions,
};
use std::{path::Path, time::Duration};
use tickets::TicketStorageManager;
use time::OffsetDateTime;
use tracing::{debug, error, log::LevelFilter};

pub mod bandwidth;
mod clients;
pub mod error;
mod inboxes;
pub mod models;
mod shared_keys;
mod tickets;
pub mod traits;
mod wireguard_peers;

pub use error::GatewayStorageError;
pub use inboxes::InboxManager;

use crate::traits::{BandwidthGatewayStorage, InboxGatewayStorage, SharedKeyGatewayStorage};

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct GatewayStorage {
    client_manager: ClientManager,
    shared_key_manager: SharedKeysManager,
    inbox_manager: InboxManager,
    bandwidth_manager: BandwidthManager,
    ticket_manager: TicketStorageManager,
    wireguard_peer_manager: wireguard_peers::WgPeerManager,
}

impl GatewayStorage {
    #[allow(dead_code)]
    pub(crate) fn client_manager(&self) -> &ClientManager {
        &self.client_manager
    }

    pub(crate) fn shared_key_manager(&self) -> &SharedKeysManager {
        &self.shared_key_manager
    }

    pub fn inbox_manager(&self) -> &InboxManager {
        &self.inbox_manager
    }

    pub(crate) fn bandwidth_manager(&self) -> &BandwidthManager {
        &self.bandwidth_manager
    }

    #[allow(dead_code)]
    pub(crate) fn ticket_manager(&self) -> &TicketStorageManager {
        &self.ticket_manager
    }

    #[allow(dead_code)]
    pub(crate) fn wireguard_peer_manager(&self) -> &wireguard_peers::WgPeerManager {
        &self.wireguard_peer_manager
    }

    pub async fn handle_forget_me(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), GatewayStorageError> {
        let client_id = self.get_mixnet_client_id(client_address).await?;
        self.inbox_manager()
            .remove_messages_for_client(&client_address.as_base58_string())
            .await?;
        self.bandwidth_manager().remove_client(client_id).await?;
        self.shared_key_manager()
            .remove_shared_keys(&client_address.as_base58_string())
            .await?;
        Ok(())
    }

    /// Initialises `PersistentStorage` using the provided path.
    ///
    /// # Arguments
    ///
    /// * `database_path`: path to the database.
    /// * `message_retrieval_limit`: maximum number of stored client messages that can be retrieved at once.
    pub async fn init<P: AsRef<Path> + Send>(
        database_path: P,
        message_retrieval_limit: i64,
    ) -> Result<Self, GatewayStorageError> {
        debug!(
            "Attempting to connect to database {}",
            database_path.as_ref().display()
        );

        // TODO: we can inject here more stuff based on our gateway global config
        // struct. Maybe different pool size or timeout intervals?
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .log_slow_statements(LevelFilter::WARN, Duration::from_millis(250))
            .filename(database_path)
            .create_if_missing(true)
            .disable_statement_logging();

        // TODO: do we want auto_vacuum ?

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
        Ok(GatewayStorage {
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
impl SharedKeyGatewayStorage for GatewayStorage {
    async fn get_mixnet_client_id(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<i64, GatewayStorageError> {
        Ok(self
            .shared_key_manager
            .client_id(&client_address.as_base58_string())
            .await?)
    }

    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &SharedGatewayKey,
    ) -> Result<i64, GatewayStorageError> {
        let client_address_bs58 = client_address.as_base58_string();
        let client_id = match self
            .shared_key_manager
            .client_id(&client_address_bs58)
            .await
        {
            Ok(client_id) => client_id,
            _ => {
                self.client_manager
                    .insert_client(ClientType::EntryMixnet)
                    .await?
            }
        };
        self.shared_key_manager
            .insert_shared_keys(
                client_id,
                client_address_bs58,
                shared_keys.aes128_ctr_hmac_bs58().as_deref(),
                shared_keys.aes256_gcm_siv().as_deref(),
            )
            .await?;
        Ok(client_id)
    }

    async fn get_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<PersistedSharedKeys>, GatewayStorageError> {
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
    ) -> Result<(), GatewayStorageError> {
        self.shared_key_manager
            .remove_shared_keys(&client_address.as_base58_string())
            .await?;
        Ok(())
    }

    async fn update_last_used_authentication_timestamp(
        &self,
        client_id: i64,
        last_used_authentication_timestamp: OffsetDateTime,
    ) -> Result<(), GatewayStorageError> {
        self.shared_key_manager
            .update_last_used_authentication_timestamp(
                client_id,
                last_used_authentication_timestamp,
            )
            .await?;
        Ok(())
    }

    async fn get_client(&self, client_id: i64) -> Result<Option<Client>, GatewayStorageError> {
        let client = self.client_manager.get_client(client_id).await?;
        Ok(client)
    }
}

#[async_trait]
impl InboxGatewayStorage for GatewayStorage {
    async fn store_message(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), GatewayStorageError> {
        self.inbox_manager
            .insert_message(&client_address.as_base58_string(), message)
            .await?;
        Ok(())
    }

    async fn retrieve_messages(
        &self,
        client_address: DestinationAddressBytes,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), GatewayStorageError> {
        let messages = self
            .inbox_manager
            .get_messages(&client_address.as_base58_string(), start_after)
            .await?;
        Ok(messages)
    }

    async fn remove_messages(&self, ids: Vec<i64>) -> Result<(), GatewayStorageError> {
        for id in ids {
            self.inbox_manager.remove_message(id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl BandwidthGatewayStorage for GatewayStorage {
    async fn create_bandwidth_entry(&self, client_id: i64) -> Result<(), GatewayStorageError> {
        self.bandwidth_manager.insert_new_client(client_id).await?;
        Ok(())
    }

    async fn set_expiration(
        &self,
        client_id: i64,
        expiration: OffsetDateTime,
    ) -> Result<(), GatewayStorageError> {
        self.bandwidth_manager
            .set_expiration(client_id, expiration)
            .await?;
        Ok(())
    }

    async fn reset_bandwidth(&self, client_id: i64) -> Result<(), GatewayStorageError> {
        self.bandwidth_manager.reset_bandwidth(client_id).await?;
        Ok(())
    }

    async fn get_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<Option<PersistedBandwidth>, GatewayStorageError> {
        Ok(self
            .bandwidth_manager
            .get_available_bandwidth(client_id)
            .await?)
    }

    async fn increase_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, GatewayStorageError> {
        Ok(self
            .bandwidth_manager
            .increase_bandwidth(client_id, amount)
            .await?)
    }

    async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        amount: i64,
    ) -> Result<(), GatewayStorageError> {
        Ok(self
            .bandwidth_manager
            .revoke_ticket_bandwidth(ticket_id, amount)
            .await?)
    }

    async fn decrease_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, GatewayStorageError> {
        Ok(self
            .bandwidth_manager
            .decrease_bandwidth(client_id, amount)
            .await?)
    }

    async fn insert_epoch_signers(
        &self,
        epoch_id: i64,
        signer_ids: Vec<i64>,
    ) -> Result<(), GatewayStorageError> {
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
    ) -> Result<i64, GatewayStorageError> {
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

    async fn contains_ticket(&self, serial_number: &[u8]) -> Result<bool, GatewayStorageError> {
        Ok(self.ticket_manager.has_ticket_data(serial_number).await?)
    }

    async fn insert_ticket_verification(
        &self,
        ticket_id: i64,
        signer_id: i64,
        verified_at: OffsetDateTime,
        accepted: bool,
    ) -> Result<(), GatewayStorageError> {
        self.ticket_manager
            .insert_ticket_verification(ticket_id, signer_id, verified_at, accepted)
            .await?;
        Ok(())
    }

    async fn update_rejected_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError> {
        // set the ticket as rejected
        self.ticket_manager.set_rejected_ticket(ticket_id).await?;

        // drop all ticket_data - we no longer need it
        // TODO: or maybe we do as a proof of receiving bad data?
        self.ticket_manager.remove_ticket_data(ticket_id).await?;

        Ok(())
    }

    async fn update_verified_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError> {
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

    async fn remove_verified_ticket_binary_data(
        &self,
        ticket_id: i64,
    ) -> Result<(), GatewayStorageError> {
        self.ticket_manager
            .remove_binary_ticket_data(ticket_id)
            .await?;
        Ok(())
    }

    async fn get_all_verified_tickets_with_sn(
        &self,
    ) -> Result<Vec<VerifiedTicket>, GatewayStorageError> {
        Ok(self
            .ticket_manager
            .get_all_verified_tickets_with_sn()
            .await?)
    }

    async fn get_all_proposed_tickets_with_sn(
        &self,
        proposal_id: u32,
    ) -> Result<Vec<VerifiedTicket>, GatewayStorageError> {
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
    ) -> Result<(), GatewayStorageError> {
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
    ) -> Result<(), GatewayStorageError> {
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

    async fn latest_proposal(&self) -> Result<Option<RedemptionProposal>, GatewayStorageError> {
        Ok(self.ticket_manager.get_latest_redemption_proposal().await?)
    }

    async fn get_all_unverified_tickets(&self) -> Result<Vec<ClientTicket>, GatewayStorageError> {
        self.ticket_manager
            .get_unverified_tickets()
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    async fn get_all_unresolved_proposals(&self) -> Result<Vec<i64>, GatewayStorageError> {
        Ok(self
            .ticket_manager
            .get_all_unresolved_redemption_proposal_ids()
            .await?)
    }

    async fn get_votes(&self, ticket_id: i64) -> Result<Vec<i64>, GatewayStorageError> {
        Ok(self
            .ticket_manager
            .get_verification_votes(ticket_id)
            .await?)
    }

    async fn get_signers(&self, epoch_id: i64) -> Result<Vec<i64>, GatewayStorageError> {
        Ok(self.ticket_manager.get_epoch_signers(epoch_id).await?)
    }

    /// Insert a wireguard peer in the storage.
    ///
    /// # Arguments
    ///
    /// * `peer`: wireguard peer data to be stored
    async fn insert_wireguard_peer(
        &self,
        peer: &defguard_wireguard_rs::host::Peer,
        client_type: ClientType,
    ) -> Result<i64, GatewayStorageError> {
        let client_id = match self
            .wireguard_peer_manager
            .retrieve_peer(&peer.public_key.to_string())
            .await?
        {
            Some(peer) => peer.client_id,
            None => self.client_manager.insert_client(client_type).await?,
        };
        let peer = WireguardPeer::from_defguard_peer(peer.clone(), client_id)?;
        self.wireguard_peer_manager.insert_peer(&peer).await?;
        Ok(client_id)
    }

    /// Tries to retrieve available bandwidth for the particular peer.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be retrieved.
    async fn get_wireguard_peer(
        &self,
        peer_public_key: &str,
    ) -> Result<Option<WireguardPeer>, GatewayStorageError> {
        let peer = self
            .wireguard_peer_manager
            .retrieve_peer(peer_public_key)
            .await?;
        Ok(peer)
    }

    /// Retrieves all wireguard peers.
    async fn get_all_wireguard_peers(&self) -> Result<Vec<WireguardPeer>, GatewayStorageError> {
        let ret = self.wireguard_peer_manager.retrieve_all_peers().await?;
        Ok(ret)
    }

    /// Remove a wireguard peer from the storage.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be removed.
    async fn remove_wireguard_peer(
        &self,
        peer_public_key: &str,
    ) -> Result<(), GatewayStorageError> {
        self.wireguard_peer_manager
            .remove_peer(peer_public_key)
            .await?;
        Ok(())
    }
}
