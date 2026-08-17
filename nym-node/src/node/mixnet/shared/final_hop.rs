// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway::node::{
    ActiveClientsStore, GatewayStorage, GatewayStorageError, InboxGatewayStorage,
};
use nym_sphinx_types::DestinationAddressBytes;
use tokio::time::Instant;
use tracing::{debug, warn};

/// What happened to a final-hop payload.
pub(crate) enum FinalHopResult {
    /// Pushed straight into the recipient's live session.
    Delivered,

    /// No live session, so it went to the recipient's on-disk inbox. The inbox holds no reference
    /// to `shared_keys`, so this accepts a recipient that never registered here too, whose row is
    /// then never collected.
    // NOTE: this will be eventually removed
    Stored,

    /// No live session, and the store rejected it.
    StoreFailed(GatewayStorageError),

    /// Came from a client with no live session, so it was neither delivered nor persisted.
    DroppedNoSession,
}

#[derive(Clone)]
pub(crate) struct SharedFinalHopData {
    active_clients: ActiveClientsStore,
    storage: GatewayStorage,
}

impl SharedFinalHopData {
    pub fn new(active_clients: ActiveClientsStore, storage: GatewayStorage) -> Self {
        Self {
            active_clients,
            storage,
        }
    }

    /// Push a final-hop payload into the recipient's live session, falling back to their on-disk
    /// inbox - except for a network monitor's packet, which is dropped instead of persisted.
    ///
    /// The monitor's agent scores a probe on what arrived on its socket, so a packet that missed
    /// the session must be definitively undelivered rather than waiting in an inbox nobody reads,
    /// and monitor traffic must not accrue undeliverable rows on every gateway in the network.
    pub(crate) async fn deliver_final_hop(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
        network_monitor_packet: bool,
    ) -> FinalHopResult {
        let unsent = match self.try_push_message_to_client(client_address, message) {
            Ok(()) => return FinalHopResult::Delivered,
            Err(unsent) => unsent,
        };

        if network_monitor_packet {
            return FinalHopResult::DroppedNoSession;
        }

        match self
            .store_processed_packet_payload(client_address, unsent)
            .await
        {
            Ok(stored) => {
                if stored {
                    FinalHopResult::Stored
                } else {
                    FinalHopResult::DroppedNoSession
                }
            }
            Err(err) => FinalHopResult::StoreFailed(err),
        }
    }

    pub(crate) fn try_push_message_to_client(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), Vec<u8>> {
        match self.active_clients.get_sender(client_address) {
            None => {
                debug!(
                    event = "gateway.push_to_client",
                    client_found = false,
                    send_result = "client_not_found",
                    "client {client_address} not found in active clients"
                );
                Err(message)
            }
            Some(sender_channel) => {
                let send_start = Instant::now();
                if let Err(unsent) = sender_channel.unbounded_send(vec![message]) {
                    warn!(
                        event = "gateway.push_to_client",
                        client_found = true,
                        send_result = "channel_closed",
                        send_us = send_start.elapsed().as_micros() as u64,
                        "client {client_address} channel closed, message not delivered"
                    );
                    // the unwrap here is fine as the original message got returned;
                    // plus we're only ever sending 1 message at the time (for now)
                    #[allow(clippy::unwrap_used)]
                    Err(unsent.into_inner().pop().unwrap())
                } else {
                    debug!(
                        event = "gateway.push_to_client",
                        client_found = true,
                        send_result = "ok",
                        send_us = send_start.elapsed().as_micros() as u64,
                        "pushed message to client {client_address}"
                    );
                    Ok(())
                }
            }
        }
    }

    /// Returns whether the payload got stored; see [`InboxGatewayStorage::store_message`].
    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<bool, GatewayStorageError> {
        let start = Instant::now();
        debug!("Storing received message for {client_address} on the disk...",);
        let result = self.storage.store_message(client_address, message).await;
        let store_us = start.elapsed().as_micros() as u64;
        match &result {
            Ok(true) => debug!(
                event = "gateway.disk_store",
                store_us, "stored message for {client_address} on disk in {store_us}us"
            ),
            Ok(false) => debug!(
                event = "gateway.disk_store_skipped",
                store_us,
                "not storing message for {client_address}: never registered with this gateway"
            ),
            Err(_) => warn!(
                event = "gateway.disk_store_failed",
                store_us, "failed to store message for {client_address} on disk after {store_us}us"
            ),
        }

        result
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use nym_sphinx_types::DESTINATION_ADDRESS_LENGTH;

    fn recipient() -> DestinationAddressBytes {
        DestinationAddressBytes::from_bytes([42u8; DESTINATION_ADDRESS_LENGTH])
    }

    /// Final hop data over an in-memory store whose active-clients store is empty, so every push
    /// fails and the fallback decision is the thing under test.
    async fn no_live_sessions() -> SharedFinalHopData {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory SQLite pool");
        let storage = GatewayStorage::from_connection_pool(pool, 100)
            .await
            .expect("failed to initialise gateway storage");

        SharedFinalHopData::new(ActiveClientsStore::new(), storage)
    }

    async fn inbox_of(final_hop: &SharedFinalHopData) -> Vec<Vec<u8>> {
        final_hop
            .storage
            .retrieve_messages(recipient(), None)
            .await
            .unwrap()
            .0
            .into_iter()
            .map(|stored| stored.content)
            .collect()
    }

    #[tokio::test]
    async fn monitor_packet_with_no_session_is_dropped_without_touching_the_store() {
        let final_hop = no_live_sessions().await;

        let result = final_hop
            .deliver_final_hop(recipient(), b"probe".to_vec(), true)
            .await;

        assert!(matches!(result, FinalHopResult::DroppedNoSession));
        assert!(inbox_of(&final_hop).await.is_empty());
    }

    #[tokio::test]
    async fn ordinary_packet_with_no_session_is_dropped_without_touching_the_store() {
        let final_hop = no_live_sessions().await;

        let result = final_hop
            .deliver_final_hop(recipient(), b"payload".to_vec(), false)
            .await;

        assert!(matches!(result, FinalHopResult::DroppedNoSession));
        assert!(inbox_of(&final_hop).await.is_empty());
    }
}
