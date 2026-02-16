// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway::node::{
    ActiveClientsStore, GatewayStorage, GatewayStorageError, InboxGatewayStorage,
};
use nym_sphinx_types::DestinationAddressBytes;
use tokio::time::Instant;
use tracing::{debug, warn};

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
                if let Err(unsent) = sender_channel.unbounded_send(vec![message]) {
                    warn!(
                        event = "gateway.push_to_client",
                        client_found = true,
                        send_result = "channel_closed",
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
                        "pushed message to client {client_address}"
                    );
                    Ok(())
                }
            }
        }
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), GatewayStorageError> {
        let start = Instant::now();
        debug!("Storing received message for {client_address} on the disk...",);
        let result = self.storage.store_message(client_address, message).await;
        let store_ms = start.elapsed().as_millis() as u64;
        if result.is_ok() {
            debug!(
                event = "gateway.disk_store",
                store_ms,
                "stored message for {client_address} on disk in {store_ms}ms"
            );
        } else {
            warn!(
                event = "gateway.disk_store_failed",
                store_ms,
                "failed to store message for {client_address} on disk after {store_ms}ms"
            );
        }
        result
    }
}
