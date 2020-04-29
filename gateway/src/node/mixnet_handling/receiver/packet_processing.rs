// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::node::client_handling::clients_handler::{
    ClientsHandlerRequest, ClientsHandlerRequestSender, ClientsHandlerResponse,
};
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use crate::node::storage::inboxes::{ClientStorage, StoreData};
use crypto::encryption;
use futures::channel::oneshot;
use futures::lock::Mutex;
use log::*;
use mix_client::packet::LOOP_COVER_MESSAGE_PAYLOAD;
use nymsphinx::{DestinationAddressBytes, Error as SphinxError, ProcessedPacket, SphinxPacket};
use std::collections::HashMap;
use std::io;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    ReceivedForwardHopError,
    NonMatchingRecipient,
    InvalidPayload,
    SphinxProcessingError(SphinxError),
    IOError(String),
}

impl From<SphinxError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of SphinxError
    fn from(err: SphinxError) -> Self {
        use MixProcessingError::*;

        SphinxProcessingError(err)
    }
}

impl From<io::Error> for MixProcessingError {
    fn from(e: io::Error) -> Self {
        use MixProcessingError::*;

        IOError(e.to_string())
    }
}

// PacketProcessor contains all data required to correctly unwrap and store sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    secret_key: Arc<encryption::PrivateKey>,
    // TODO: later investigate some concurrent hashmap solutions or perhaps RWLocks.
    // Right now Mutex is the simplest and fastest to implement approach
    available_socket_senders_cache: Arc<Mutex<HashMap<DestinationAddressBytes, MixMessageSender>>>,
    client_store: ClientStorage,
    clients_handler_sender: ClientsHandlerRequestSender,
}

impl PacketProcessor {
    pub(crate) fn new(
        secret_key: Arc<encryption::PrivateKey>,
        clients_handler_sender: ClientsHandlerRequestSender,
        client_store: ClientStorage,
    ) -> Self {
        PacketProcessor {
            available_socket_senders_cache: Arc::new(Mutex::new(HashMap::new())),
            clients_handler_sender,
            client_store,
            secret_key,
        }
    }

    fn try_push_message_to_client(
        &self,
        sender_channel: Option<MixMessageSender>,
        message: Vec<u8>,
    ) -> bool {
        match sender_channel {
            None => false,
            Some(sender_channel) => sender_channel.unbounded_send(vec![message]).is_ok(),
        }
    }

    async fn try_to_obtain_client_ws_message_sender(
        &mut self,
        client_address: DestinationAddressBytes,
    ) -> Option<MixMessageSender> {
        let mut cache_guard = self.available_socket_senders_cache.lock().await;

        if let Some(sender) = cache_guard.get(&client_address) {
            if !sender.is_closed() {
                return Some(sender.clone());
            } else {
                cache_guard.remove(&client_address);
            }
        }

        // do not block other readers to the cache while we are doing some blocking work here
        drop(cache_guard);

        // if we got here it means that either we have no sender channel for this client or it's closed
        // so we must refresh it from the source, i.e. ClientsHandler
        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request =
            ClientsHandlerRequest::IsOnline(client_address.clone(), res_sender);
        self.clients_handler_sender
            .unbounded_send(clients_handler_request)
            .unwrap(); // the receiver MUST BE alive

        let client_sender = match res_receiver.await.unwrap() {
            ClientsHandlerResponse::IsOnline(client_sender) => client_sender,
            _ => panic!("received response to wrong query!"), // again, this should NEVER happen
        };

        if client_sender.is_none() {
            return None;
        }

        let client_sender = client_sender.unwrap();
        // finally re-acquire the lock to update the cache
        let mut cache_guard = self.available_socket_senders_cache.lock().await;
        cache_guard.insert(client_address, client_sender.clone());

        Some(client_sender)
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> io::Result<()> {
        trace!(
            "Storing received packet for {:?} on the disk...",
            client_address.to_base58_string()
        );
        // we are temporarily ignoring and not storing obvious loop cover traffic messages to
        // not cause our sfw-provider to run out of disk space too quickly.
        // Eventually this is going to get removed and be replaced by a quota system described in:
        // https://github.com/nymtech/nym/issues/137
        if message == LOOP_COVER_MESSAGE_PAYLOAD {
            debug!("Received a loop cover message - not going to store it");
            return Ok(());
        }

        let store_data = StoreData::new(client_address, message);
        self.client_store.store_processed_data(store_data).await
    }

    pub(crate) fn unwrap_sphinx_packet(
        &self,
        raw_packet_data: [u8; nymsphinx::PACKET_SIZE],
    ) -> Result<(DestinationAddressBytes, Vec<u8>), MixProcessingError> {
        let packet = SphinxPacket::from_bytes(&raw_packet_data)?;

        match packet.process(self.secret_key.deref().inner()) {
            Ok(ProcessedPacket::ProcessedPacketForwardHop(_, _, _)) => {
                warn!("Received a forward hop message - those are not implemented for providers");
                Err(MixProcessingError::ReceivedForwardHopError)
            }
            Ok(ProcessedPacket::ProcessedPacketFinalHop(client_address, _surb_id, payload)) => {
                // in our current design, we do not care about the 'surb_id' in the header
                // as it will always be empty anyway
                let (payload_destination, message) = payload
                    .try_recover_destination_and_plaintext()
                    .ok_or_else(|| MixProcessingError::InvalidPayload)?;
                if client_address != payload_destination {
                    return Err(MixProcessingError::NonMatchingRecipient);
                }
                Ok((client_address, message))
            }
            Err(e) => {
                warn!("Failed to unwrap Sphinx packet: {:?}", e);
                Err(MixProcessingError::SphinxProcessingError(e))
            }
        }
    }

    pub(crate) async fn process_sphinx_packet(
        &mut self,
        raw_packet_data: [u8; nymsphinx::PACKET_SIZE],
    ) -> Result<(), MixProcessingError> {
        let (client_address, plaintext) = self.unwrap_sphinx_packet(raw_packet_data)?;

        let client_sender = self
            .try_to_obtain_client_ws_message_sender(client_address.clone())
            .await;

        // TODO: think of a way to prevent having to clone the plaintext here, perhaps make channels use references?
        // this will, again, take slightly more time, so it's an issue for later
        if !self.try_push_message_to_client(client_sender, plaintext.clone()) {
            Ok(self
                .store_processed_packet_payload(client_address, plaintext)
                .await?)
        } else {
            trace!(
                "Managed to push received packet for {:?} to websocket connection!",
                client_address.to_base58_string()
            );
            Ok(())
        }
    }
}
