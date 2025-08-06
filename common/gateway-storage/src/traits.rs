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

    /// Tries to retrieve a particular peer with the given public key.
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

#[cfg(feature = "mock")]
pub mod mock {
    use std::{collections::HashMap, sync::Arc};

    use tokio::sync::RwLock;

    use super::*;

    struct EcashSigner {
        _epoch_id: i64,
        _signer_id: i64,
    }

    struct ReceivedTicket {
        client_id: i64,
        _received_at: OffsetDateTime,
        rejected: Option<bool>,
    }

    struct TicketData {
        serial_number: Vec<u8>,
        data: Option<Vec<u8>>,
    }

    struct TicketVerification {
        _ticket_id: i64,
        _signer_id: i64,
        _verified_at: OffsetDateTime,
        _accepted: bool,
    }

    #[derive(Default)]
    pub struct MockGatewayStorage {
        available_bandwidth: HashMap<i64, PersistedBandwidth>,
        ecash_signers: Vec<EcashSigner>,
        received_ticket: HashMap<i64, ReceivedTicket>,
        ticket_data: HashMap<i64, TicketData>,
        ticket_verification: HashMap<i64, TicketVerification>,
        verified_tickets: Vec<i64>,
        wireguard_peers: HashMap<String, WireguardPeer>,
        clients: HashMap<i64, String>,
    }

    #[async_trait]
    impl BandwidthGatewayStorage for Arc<RwLock<MockGatewayStorage>> {
        async fn create_bandwidth_entry(&self, client_id: i64) -> Result<(), GatewayStorageError> {
            self.write().await.available_bandwidth.insert(
                client_id,
                PersistedBandwidth {
                    client_id,
                    available: 0,
                    expiration: Some(OffsetDateTime::UNIX_EPOCH),
                },
            );
            Ok(())
        }

        async fn set_expiration(
            &self,
            client_id: i64,
            expiration: OffsetDateTime,
        ) -> Result<(), GatewayStorageError> {
            if let Some(bw) = self.write().await.available_bandwidth.get_mut(&client_id) {
                bw.expiration = Some(expiration);
            }
            Ok(())
        }

        async fn reset_bandwidth(&self, client_id: i64) -> Result<(), GatewayStorageError> {
            if let Some(bw) = self.write().await.available_bandwidth.get_mut(&client_id) {
                bw.available = 0;
                bw.expiration = Some(OffsetDateTime::UNIX_EPOCH);
            }
            Ok(())
        }

        async fn get_available_bandwidth(
            &self,
            client_id: i64,
        ) -> Result<Option<PersistedBandwidth>, GatewayStorageError> {
            Ok(self
                .read()
                .await
                .available_bandwidth
                .get(&client_id)
                .cloned())
        }

        async fn increase_bandwidth(
            &self,
            client_id: i64,
            amount: i64,
        ) -> Result<i64, GatewayStorageError> {
            self.write()
                .await
                .available_bandwidth
                .get_mut(&client_id)
                .map(|bw| {
                    bw.available += amount;
                    bw.available
                })
                .ok_or(GatewayStorageError::InternalDatabaseError(
                    sqlx::Error::RowNotFound,
                ))
        }

        async fn revoke_ticket_bandwidth(
            &self,
            ticket_id: i64,
            amount: i64,
        ) -> Result<(), GatewayStorageError> {
            let mut guard = self.write().await;
            if let Some(client_id) = guard
                .received_ticket
                .get(&ticket_id)
                .map(|ticket| ticket.client_id)
            {
                if let Some(bw) = guard.available_bandwidth.get_mut(&client_id) {
                    bw.available -= amount;
                }
            }
            Ok(())
        }

        async fn decrease_bandwidth(
            &self,
            client_id: i64,
            amount: i64,
        ) -> Result<i64, GatewayStorageError> {
            self.write()
                .await
                .available_bandwidth
                .get_mut(&client_id)
                .map(|bw| {
                    bw.available -= amount;
                    bw.available
                })
                .ok_or(GatewayStorageError::InternalDatabaseError(
                    sqlx::Error::RowNotFound,
                ))
        }

        async fn insert_epoch_signers(
            &self,
            _epoch_id: i64,
            signer_ids: Vec<i64>,
        ) -> Result<(), GatewayStorageError> {
            self.write()
                .await
                .ecash_signers
                .extend(signer_ids.iter().map(|signer_id| EcashSigner {
                    _epoch_id,
                    _signer_id: *signer_id,
                }));
            Ok(())
        }

        async fn insert_received_ticket(
            &self,
            client_id: i64,
            _received_at: OffsetDateTime,
            serial_number: Vec<u8>,
            data: Vec<u8>,
        ) -> Result<i64, GatewayStorageError> {
            let mut guard = self.write().await;
            let ticket_id = guard.received_ticket.len() as i64;
            guard.received_ticket.insert(
                ticket_id,
                ReceivedTicket {
                    client_id,
                    _received_at,
                    rejected: None,
                },
            );
            guard.ticket_data.insert(
                ticket_id,
                TicketData {
                    serial_number,
                    data: Some(data),
                },
            );
            Ok(ticket_id)
        }

        async fn contains_ticket(&self, serial_number: &[u8]) -> Result<bool, GatewayStorageError> {
            Ok(self
                .read()
                .await
                .ticket_data
                .values()
                .any(|ticket_data| ticket_data.serial_number == serial_number))
        }

        async fn insert_ticket_verification(
            &self,
            _ticket_id: i64,
            _signer_id: i64,
            _verified_at: OffsetDateTime,
            _accepted: bool,
        ) -> Result<(), GatewayStorageError> {
            self.write().await.ticket_verification.insert(
                _ticket_id,
                TicketVerification {
                    _ticket_id,
                    _signer_id,
                    _verified_at,
                    _accepted,
                },
            );
            Ok(())
        }

        async fn update_rejected_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError> {
            let mut guard = self.write().await;
            if let Some(ticket) = guard.received_ticket.get_mut(&ticket_id) {
                ticket.rejected = Some(true);
            }
            guard.ticket_data.remove(&ticket_id);
            Ok(())
        }

        async fn update_verified_ticket(&self, ticket_id: i64) -> Result<(), GatewayStorageError> {
            let mut guard = self.write().await;
            guard.verified_tickets.push(ticket_id);
            guard.ticket_verification.remove(&ticket_id);
            Ok(())
        }

        async fn remove_verified_ticket_binary_data(
            &self,
            ticket_id: i64,
        ) -> Result<(), GatewayStorageError> {
            if let Some(ticket) = self.write().await.ticket_data.get_mut(&ticket_id) {
                ticket.data = None;
            }
            Ok(())
        }

        async fn get_all_verified_tickets_with_sn(
            &self,
        ) -> Result<Vec<VerifiedTicket>, GatewayStorageError> {
            todo!()
        }

        async fn get_all_proposed_tickets_with_sn(
            &self,
            _proposal_id: u32,
        ) -> Result<Vec<VerifiedTicket>, GatewayStorageError> {
            todo!()
        }

        async fn insert_redemption_proposal(
            &self,
            _tickets: &[VerifiedTicket],
            _proposal_id: u32,
            _created_at: OffsetDateTime,
        ) -> Result<(), GatewayStorageError> {
            todo!()
        }

        async fn clear_post_proposal_data(
            &self,
            _proposal_id: u32,
            _resolved_at: OffsetDateTime,
            _rejected: bool,
        ) -> Result<(), GatewayStorageError> {
            todo!()
        }

        async fn latest_proposal(&self) -> Result<Option<RedemptionProposal>, GatewayStorageError> {
            todo!()
        }

        async fn get_all_unverified_tickets(
            &self,
        ) -> Result<Vec<ClientTicket>, GatewayStorageError> {
            todo!()
        }

        async fn get_all_unresolved_proposals(&self) -> Result<Vec<i64>, GatewayStorageError> {
            todo!()
        }

        async fn get_votes(&self, _ticket_id: i64) -> Result<Vec<i64>, GatewayStorageError> {
            todo!()
        }

        async fn get_signers(&self, _epoch_id: i64) -> Result<Vec<i64>, GatewayStorageError> {
            todo!()
        }

        async fn insert_wireguard_peer(
            &self,
            peer: &defguard_wireguard_rs::host::Peer,
            client_type: ClientType,
        ) -> Result<i64, GatewayStorageError> {
            let mut guard = self.write().await;
            let client_id =
                if let Some(peer) = guard.wireguard_peers.get(&peer.public_key.to_string()) {
                    peer.client_id
                } else {
                    let client_id = guard.clients.len() as i64;
                    guard.clients.insert(client_id, client_type.to_string());
                    client_id
                };
            guard.wireguard_peers.insert(
                peer.public_key.to_string(),
                WireguardPeer::from_defguard_peer(peer.clone(), client_id)?,
            );
            Ok(client_id)
        }

        async fn get_wireguard_peer(
            &self,
            peer_public_key: &str,
        ) -> Result<Option<WireguardPeer>, GatewayStorageError> {
            Ok(self
                .read()
                .await
                .wireguard_peers
                .get(peer_public_key)
                .cloned())
        }

        async fn get_all_wireguard_peers(&self) -> Result<Vec<WireguardPeer>, GatewayStorageError> {
            todo!()
        }

        async fn remove_wireguard_peer(
            &self,
            peer_public_key: &str,
        ) -> Result<(), GatewayStorageError> {
            self.write().await.wireguard_peers.remove(peer_public_key);
            Ok(())
        }
    }
}
