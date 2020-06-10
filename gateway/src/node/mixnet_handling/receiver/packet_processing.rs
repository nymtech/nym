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
use crate::node::mixnet_handling::sender::OutboundMixMessageSender;
use crate::node::storage::inboxes::{ClientStorage, StoreData};
use crypto::encryption;
use futures::channel::oneshot;
use futures::lock::Mutex;
use log::*;
use nymsphinx::acknowledgements::surb_ack::{SURBAck, SURBAckRecoveryError};
use nymsphinx::cover::LOOP_COVER_MESSAGE_PAYLOAD;
use nymsphinx::params::packet_sizes::PacketSize;
use nymsphinx::{DestinationAddressBytes, Error as SphinxError, ProcessedPacket, SphinxPacket};

use std::collections::HashMap;
use std::io;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    ReceivedForwardHopError,
    NonMatchingRecipient,
    UnsupportedSphinxPacketSize(usize),
    SphinxProcessingError(SphinxError),
    IncorrectlyFormattedSURBAck(SURBAckRecoveryError),
    IOError(io::Error),
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

        IOError(e)
    }
}

impl From<SURBAckRecoveryError> for MixProcessingError {
    fn from(err: SURBAckRecoveryError) -> Self {
        use MixProcessingError::*;

        IncorrectlyFormattedSURBAck(err)
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
    ack_sender: OutboundMixMessageSender,
}

impl PacketProcessor {
    pub(crate) fn new(
        secret_key: Arc<encryption::PrivateKey>,
        clients_handler_sender: ClientsHandlerRequestSender,
        client_store: ClientStorage,
        ack_sender: OutboundMixMessageSender,
    ) -> Self {
        PacketProcessor {
            available_socket_senders_cache: Arc::new(Mutex::new(HashMap::new())),
            clients_handler_sender,
            client_store,
            secret_key,
            ack_sender,
        }
    }

    fn try_push_message_to_client(
        &self,
        sender_channel: Option<MixMessageSender>,
        message: Vec<u8>,
    ) -> Result<(), Vec<u8>> {
        match sender_channel {
            None => Err(message),
            Some(sender_channel) => {
                sender_channel
                    .unbounded_send(vec![message])
                    // right now it's a "simpler" case here as we're only ever sending 1 message
                    // at the time, but the channel itself could accept arbitrary many messages at once
                    .map_err(|try_send_err| try_send_err.into_inner().pop().unwrap())
            }
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
        debug!(
            "Storing received packet for {:?} on the disk...",
            client_address.to_base58_string()
        );
        // we are temporarily ignoring and not storing obvious loop cover traffic messages to
        // not cause our sfw-provider to run out of disk space too quickly.
        // Eventually this is going to get removed and be replaced by a quota system described in:
        // https://github.com/nymtech/nym/issues/137

        // JS: I think this would never get called anyway, because if loop cover messages are sent
        // it means client is online and hence all his messages should be pushed directly to him?
        if message == LOOP_COVER_MESSAGE_PAYLOAD {
            debug!("Received a loop cover message - not going to store it");
            return Ok(());
        }

        let store_data = StoreData::new(client_address, message);
        self.client_store.store_processed_data(store_data).await
    }

    pub(crate) fn unwrap_sphinx_packet(
        &self,
        packet: SphinxPacket,
    ) -> Result<(DestinationAddressBytes, Vec<u8>), MixProcessingError> {
        match packet.process(self.secret_key.deref().inner()) {
            Ok(ProcessedPacket::ProcessedPacketForwardHop(_, _, _)) => {
                warn!("Received a forward hop message - those are not implemented for gateways");
                Err(MixProcessingError::ReceivedForwardHopError)
            }
            Ok(ProcessedPacket::ProcessedPacketFinalHop(client_address, _surb_id, payload)) => {
                // in our current design, we do not care about the 'surb_id' in the header
                // as it will always be empty anyway
                let (payload_destination, message) =
                    payload.try_recover_destination_and_plaintext()?;
                // TODO: @AP, does that check still make sense?
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

    fn split_plaintext_into_ack_and_message(
        &self,
        mut extracted_plaintext: Vec<u8>,
    ) -> (Vec<u8>, Vec<u8>) {
        if extracted_plaintext.len() < SURBAck::len() {
            // TODO:
            // TODO:
            // this is mostly for dev purposes to see if we receive something we did not mean to send
            // but in an actual system, what should we do? abandon the whole packet?
            // store client's data regardless?
            // I'm going to leave this question open for until I've implemented reply SURBs
            // as they will change the communication between client and gateway so this
            // if statement might no longer make any sense
            panic!("received packet without an ack");
        }

        let plaintext = extracted_plaintext.split_off(SURBAck::len());
        let ack_data = extracted_plaintext;
        (ack_data, plaintext)
    }

    pub(crate) async fn process_sphinx_packet(
        &mut self,
        sphinx_packet: SphinxPacket,
    ) -> Result<(), MixProcessingError> {
        // see if what we got now is an ack or normal packet
        let packet_len = sphinx_packet.len();
        // TODO: micro-optimisations:
        // 1. don't even try to unwrap the packet if it's not one of `PacketSize` variants
        // 2. if client_address doesn't exist at this gateway, don't do any other work here
        // (as stupid as this sounds, there's currently no easy way of directly checking if the
        // client exists here)
        let (client_address, plaintext) = self.unwrap_sphinx_packet(sphinx_packet)?;
        let (routable_ack, plaintext) = match packet_len {
            n if n == PacketSize::ACKPacket.size() => {
                trace!("received an ack packet!");
                (None, plaintext)
            }
            n if n == PacketSize::RegularPacket.size()
                || n == PacketSize::ExtendedPacket.size() =>
            {
                trace!("received a normal packet!");
                let (ack_data, plaintext) = self.split_plaintext_into_ack_and_message(plaintext);
                let (ack_first_hop, ack_packet) = SURBAck::try_recover_first_hop_packet(&ack_data)?;
                (Some((ack_first_hop, ack_packet)), plaintext)
            }
            n => return Err(MixProcessingError::UnsupportedSphinxPacketSize(n)),
        };

        let client_sender = self
            .try_to_obtain_client_ws_message_sender(client_address.clone())
            .await;

        if let Err(unsent_plaintext) = self.try_push_message_to_client(client_sender, plaintext) {
            // means we failed to push message directly to the client (it might be offline)
            // but we don't want to store an ack message for him - he won't be able to decode
            // it anyway.
            // TODO: after keybase discussion we *might* want to store them after all
            if routable_ack.is_none() {
                trace!("Received an ack for offline client - won't try storing it");
                return Ok(());
            }

            if let Err(io_err) = self
                .store_processed_packet_payload(client_address.clone(), unsent_plaintext)
                .await
            {
                return Err(io_err)?;
            } else {
                trace!(
                    "Managed to store packet for {:?} on the disk",
                    client_address.to_base58_string()
                );
            }
        } else {
            trace!(
                "Managed to push received packet for {:?} to websocket connection!",
                client_address.to_base58_string()
            );
        }

        // if we managed to either push message directly to the [online] client or store it at
        // it's inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        if let Some((ack_first_hop, ack_packet)) = routable_ack {
            trace!(
                "Sending an ack back into the network. The first hop is {:?}",
                ack_first_hop
            );
            self.ack_sender
                .unbounded_send((ack_first_hop.into(), ack_packet))
                .unwrap();
        }

        Ok(())
    }
}
