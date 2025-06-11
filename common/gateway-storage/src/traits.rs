// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_credentials_interface::ClientTicket;
use nym_gateway_requests::SharedGatewayKey;
use nym_sphinx::DestinationAddressBytes;
use time::OffsetDateTime;

use crate::{
    clients::ClientType,
    models::{
        Client, PersistedBandwidth, PersistedSharedKeys, RedemptionProposal, StoredMessage,
        VerifiedTicket, WireguardPeer,
    },
    GatewayStorageError,
};

#[async_trait]
pub trait SharedKeyGatewayStorage {
    async fn get_mixnet_client_id(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<i64, GatewayStorageError>;
    async fn insert_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
        shared_keys: &SharedGatewayKey,
    ) -> Result<i64, GatewayStorageError>;
    async fn get_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<Option<PersistedSharedKeys>, GatewayStorageError>;
    #[allow(dead_code)]
    async fn remove_shared_keys(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<(), GatewayStorageError>;
    async fn update_last_used_authentication_timestamp(
        &self,
        client_id: i64,
        last_used_authentication_timestamp: OffsetDateTime,
    ) -> Result<(), GatewayStorageError>;
    async fn get_client(&self, client_id: i64) -> Result<Option<Client>, GatewayStorageError>;
}

#[async_trait]
pub trait InboxGatewayStorage {
    async fn store_message(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), GatewayStorageError>;
    async fn retrieve_messages(
        &self,
        client_address: DestinationAddressBytes,
        start_after: Option<i64>,
    ) -> Result<(Vec<StoredMessage>, Option<i64>), GatewayStorageError>;
    async fn remove_messages(&self, ids: Vec<i64>) -> Result<(), GatewayStorageError>;
}

#[async_trait]
pub trait BandwidthGatewayStorage: dyn_clone::DynClone {
    async fn create_bandwidth_entry(&self, client_id: i64) -> Result<(), GatewayStorageError>;
    async fn set_expiration(
        &self,
        client_id: i64,
        expiration: OffsetDateTime,
    ) -> Result<(), GatewayStorageError>;
    async fn reset_bandwidth(&self, client_id: i64) -> Result<(), GatewayStorageError>;
    async fn get_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<Option<PersistedBandwidth>, GatewayStorageError>;
    async fn increase_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, GatewayStorageError>;
    async fn revoke_ticket_bandwidth(
        &self,
        ticket_id: i64,
        amount: i64,
    ) -> Result<(), GatewayStorageError>;
    async fn decrease_bandwidth(
        &self,
        client_id: i64,
        amount: i64,
    ) -> Result<i64, GatewayStorageError>;

    async fn insert_epoch_signers(
        &self,
        epoch_id: i64,
        signer_ids: Vec<i64>,
    ) -> Result<(), GatewayStorageError>;
    async fn insert_received_ticket(
        &self,
        client_id: i64,
        received_at: OffsetDateTime,
        serial_number: Vec<u8>,
        data: Vec<u8>,
    ) -> Result<i64, GatewayStorageError>;
    async fn contains_ticket(&self, serial_number: &[u8]) -> Result<bool, GatewayStorageError>;
    async fn insert_ticket_verification(
        &self,
        ticket_id: i64,
        signer_id: i64,
        verified_at: OffsetDateTime,
        accepted: bool,
    ) -> Result<(), GatewayStorageError>;
    async fn update_rejected_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError>;
    async fn update_verified_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError>;
    async fn remove_verified_ticket_binary_data(
        &self,
        ticket_id: i64,
    ) -> Result<(), GatewayStorageError>;
    async fn get_all_verified_tickets_with_sn(
        &self,
    ) -> Result<Vec<VerifiedTicket>, GatewayStorageError>;
    async fn get_all_proposed_tickets_with_sn(
        &self,
        proposal_id: u32,
    ) -> Result<Vec<VerifiedTicket>, GatewayStorageError>;
    async fn insert_redemption_proposal(
        &self,
        tickets: &[VerifiedTicket],
        proposal_id: u32,
        created_at: OffsetDateTime,
    ) -> Result<(), GatewayStorageError>;
    async fn clear_post_proposal_data(
        &self,
        proposal_id: u32,
        resolved_at: OffsetDateTime,
        rejected: bool,
    ) -> Result<(), GatewayStorageError>;
    async fn latest_proposal(&self) -> Result<Option<RedemptionProposal>, GatewayStorageError>;
    async fn get_all_unverified_tickets(&self) -> Result<Vec<ClientTicket>, GatewayStorageError>;
    async fn get_all_unresolved_proposals(&self) -> Result<Vec<i64>, GatewayStorageError>;
    async fn get_votes(&self, ticket_id: i64) -> Result<Vec<i64>, GatewayStorageError>;
    async fn get_signers(&self, epoch_id: i64) -> Result<Vec<i64>, GatewayStorageError>;

    /// Insert a wireguard peer in the storage.
    ///
    /// # Arguments
    ///
    /// * `peer`: wireguard peer data to be stored
    async fn insert_wireguard_peer(
        &self,
        peer: &defguard_wireguard_rs::host::Peer,
        client_type: ClientType,
    ) -> Result<i64, GatewayStorageError>;

    /// Tries to retrieve available bandwidth for the particular peer.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be retrieved.
    async fn get_wireguard_peer(
        &self,
        peer_public_key: &str,
    ) -> Result<Option<WireguardPeer>, GatewayStorageError>;

    /// Retrieves all wireguard peers.
    async fn get_all_wireguard_peers(&self) -> Result<Vec<WireguardPeer>, GatewayStorageError>;

    /// Remove a wireguard peer from the storage.
    ///
    /// # Arguments
    ///
    /// * `peer_public_key`: wireguard public key of the peer to be removed.
    async fn remove_wireguard_peer(&self, peer_public_key: &str)
        -> Result<(), GatewayStorageError>;
}
