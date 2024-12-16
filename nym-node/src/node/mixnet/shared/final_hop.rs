// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway::node::{ActiveClientsStore, GatewayStorage, GatewayStorageError};
use nym_sphinx_types::DestinationAddressBytes;
use tracing::debug;

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
            None => Err(message),
            Some(sender_channel) => {
                if let Err(unsent) = sender_channel.unbounded_send(vec![message]) {
                    // the unwrap here is fine as the original message got returned;
                    // plus we're only ever sending 1 message at the time (for now)
                    #[allow(clippy::unwrap_used)]
                    Err(unsent.into_inner().pop().unwrap())
                } else {
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
        debug!("Storing received message for {client_address} on the disk...",);

        self.storage.store_message(client_address, message).await
    }
}
