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

    /// No live session, so it went to the recipient's on-disk inbox. Only a registered recipient
    /// reaches here: the insert is gated on the recipient having a `shared_keys` row.
    // NOTE: this will be eventually removed
    Stored,

    /// No live session, and the store rejected it.
    StoreFailed(GatewayStorageError),

    /// Neither delivered nor persisted: either a monitor's packet, which is never stored, or an
    /// unregistered recipient, whose inbox insert the store declines.
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
    use nym_gateway::node::SharedKeyGatewayStorage;
    use nym_gateway_requests::SharedSymmetricKey;
    use nym_sphinx_types::DESTINATION_ADDRESS_LENGTH;

    fn recipient() -> DestinationAddressBytes {
        DestinationAddressBytes::from_bytes([42u8; DESTINATION_ADDRESS_LENGTH])
    }

    /// Gives `recipient()` a `shared_keys` row, which is what the inbox insert is gated on: without
    /// one the store rejects the message whoever sent it, so a test that skips this cannot tell the
    /// monitor drop apart from an ordinary fallback that simply had nowhere to land.
    async fn register_recipient(final_hop: &SharedFinalHopData) {
        final_hop
            .storage
            .insert_shared_keys(
                recipient(),
                &SharedSymmetricKey::try_from_bytes(&[1u8; 32]).unwrap(),
            )
            .await
            .expect("failed to register the recipient");
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

    // The pair below differs ONLY in the monitor flag: same storage, same registered recipient,
    // same call. That is what makes them evidence for the drop rather than for the registration
    // gate, and what makes removing the monitor branch fail the first of them.

    #[tokio::test]
    async fn monitor_packet_with_no_session_is_dropped_without_touching_the_store() {
        let final_hop = no_live_sessions().await;
        register_recipient(&final_hop).await;

        let result = final_hop
            .deliver_final_hop(recipient(), b"probe".to_vec(), true)
            .await;

        assert!(matches!(result, FinalHopResult::DroppedNoSession));
        assert!(inbox_of(&final_hop).await.is_empty());
    }

    #[tokio::test]
    async fn ordinary_packet_with_no_session_falls_back_to_the_inbox() {
        let final_hop = no_live_sessions().await;
        register_recipient(&final_hop).await;

        let result = final_hop
            .deliver_final_hop(recipient(), b"payload".to_vec(), false)
            .await;

        assert!(matches!(result, FinalHopResult::Stored));
        assert_eq!(inbox_of(&final_hop).await, vec![b"payload".to_vec()]);
    }

    /// The store gates the insert on a `shared_keys` row, so an unregistered recipient is dropped
    /// rather than accruing an orphan row. Retained as its own case because it is the behaviour the
    /// two tests above used to be accidentally asserting.
    #[tokio::test]
    async fn ordinary_packet_for_an_unregistered_recipient_is_dropped() {
        let final_hop = no_live_sessions().await;

        let result = final_hop
            .deliver_final_hop(recipient(), b"payload".to_vec(), false)
            .await;

        assert!(matches!(result, FinalHopResult::DroppedNoSession));
        assert!(inbox_of(&final_hop).await.is_empty());
    }
}
