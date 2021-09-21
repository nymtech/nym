// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::storage::error::StorageError;
use crate::node::storage::GatewayStorage;
use crate::node::{
    client_handling::websocket::message_receiver::MixMessageSender,
    storage::{inboxes::ClientStorage, ClientLedger},
};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::iv::IV;
use gateway_requests::registration::handshake::SharedKeys;
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::collections::HashMap;
use tokio::task::JoinHandle;

pub(crate) type ClientsHandlerRequestSender = mpsc::UnboundedSender<ClientsHandlerRequest>;
pub(crate) type ClientsHandlerRequestReceiver = mpsc::UnboundedReceiver<ClientsHandlerRequest>;

pub(crate) type ClientsHandlerResponseSender = oneshot::Sender<ClientsHandlerResponse>;

// #[derive(Debug)]
pub(crate) enum ClientsHandlerRequest {
    // client
    Register(
        DestinationAddressBytes,
        SharedKeys,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),
    Authenticate(
        DestinationAddressBytes,
        EncryptedAddressBytes,
        IV,
        MixMessageSender,
        ClientsHandlerResponseSender,
    ),
    Disconnect(DestinationAddressBytes),

    // mix
    IsOnline(DestinationAddressBytes, ClientsHandlerResponseSender),
}

#[derive(Debug)]
pub(crate) enum ClientsHandlerResponse {
    Register(bool),
    Authenticate(Option<SharedKeys>),
    IsOnline(Option<MixMessageSender>),
    Error(Box<dyn std::error::Error + Send + Sync>),
}

pub(crate) struct ClientsHandler {
    open_connections: HashMap<DestinationAddressBytes, MixMessageSender>,
    storage: GatewayStorage,
}

impl ClientsHandler {
    pub(crate) fn new(storage: GatewayStorage) -> Self {
        ClientsHandler {
            open_connections: HashMap::new(),
            storage,
        }
    }

    fn make_error_response<E>(&self, err: E) -> ClientsHandlerResponse
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        ClientsHandlerResponse::Error(err.into())
    }

    // best effort sending error responses
    fn send_error_response<E>(&self, err: E, res_channel: ClientsHandlerResponseSender)
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        if res_channel.send(self.make_error_response(err)).is_err() {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    async fn push_stored_messages_to_client_and_save_channel(
        &mut self,
        client_address: DestinationAddressBytes,
        comm_channel: MixMessageSender,
    ) {
        // TODO: it is possible that during a small window some of client messages will be "lost",
        // i.e. be stored on the disk rather than pushed to the client, reason for this is as follows:
        // now we push all stored messages from client's inbox to its websocket connection
        // however, say, at the same time there's new message to the client - it gets stored on the disk!
        // And only after this method exits, mix receivers will become aware of the client
        // connection going online and being able to forward traffic there.
        //
        // possible solution: spawn a future to empty inbox in X seconds rather than immediately
        // JS: I will most likely do that (with including entries to config, etc.) once the
        // basic version is up and running as not to waste time on it now
        //
        // possible solution2 after a year: just have an atomic flag to indicate stuff should cache messages for few seconds

        // TODO: SECURITY (kinda):
        // We should stagger reading the messages in a different way, i.e. we read some of them,
        // send them all the way back to the client and then read next batch. Otherwise we risk
        // being vulnerable to trivial attacks causing gateway crashes.
        let mut start_next_after = None;
        loop {
            let (messages, new_start_next_after) = match self
                .storage
                .retrieve_messages(client_address, start_next_after)
                .await
            {
                Err(err) => {
                    error!(
                    "failed to retrieve client messages - {}. There might be some database corruption.",
                    err
                );
                    return;
                }
                Ok(stored) => stored,
            };

            let (messages, ids) = messages
                .into_iter()
                .map(|msg| (msg.content, msg.id))
                .unzip();

            if comm_channel.unbounded_send(messages).is_err() {
                error!("Somehow we failed to stored messages to a fresh client channel - there seem to be a weird bug present!");
            } else {
                // after sending the messages, remove them from the storage
                // TODO: this kinda relates to the previously mentioned idea of different staggering method
                // because technically we don't know if the client received those messages. We only pushed
                // them upon the channel that will eventually be read and then sent to the socket
                // so technically we can lose packets here
                if let Err(err) = self.storage.remove_messages(ids).await {
                    error!(
                        "failed to remove old client messages - {}. There might be some database corruption.",
                    err
                    );
                }
            }

            // no more messages to grab
            if new_start_next_after.is_none() {
                break;
            } else {
                start_next_after = new_start_next_after
            }
        }

        // finally, everything was fine - we retrieved everything, we deleted everything,
        // we assume we can now safely delegate client message pushing
        self.open_connections.insert(client_address, comm_channel);
    }

    async fn handle_register_request(
        &mut self,
        address: DestinationAddressBytes,
        derived_shared_key: SharedKeys,
        comm_channel: MixMessageSender,
        res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing register new client request: {}",
            address.as_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!(
                "Tried to process register request for a client with an already opened connection!"
            );
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        if let Err(err) = self
            .storage
            .insert_shared_keys(address, derived_shared_key)
            .await
        {
            error!("We failed to store client's shared key... - {}", err);
            self.send_error_response("Internal gateway storage error", res_channel);
            return;
        }

        self.push_stored_messages_to_client_and_save_channel(address, comm_channel)
            .await;

        if res_channel
            .send(ClientsHandlerResponse::Register(true))
            .is_err()
        {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    /// Checks whether the stored shared keys match the received data, i.e. whether the upon decryption
    /// the provided encrypted address matches the expected unencrypted address.
    ///
    /// Returns the result of the check alongside the retrieved shared key,
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client.
    /// * `encrypted_address`: encrypted address of the client, presumably encrypted using the shared keys.
    /// * `iv`: iv created for this particular encryption.
    async fn verify_stored_shared_key(
        &self,
        client_address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        iv: IV,
    ) -> Result<(bool, Option<SharedKeys>), StorageError> {
        let shared_keys = self.storage.get_shared_keys(client_address).await?;

        if let Some(shared_keys) = shared_keys {
            // the unwrap here is fine as we only ever construct persisted shared keys ourselves when inserting
            // data to the storage. The only way it could fail is if we somehow changed implementation without
            // performing proper migration
            let keys = SharedKeys::try_from_base58_string(
                shared_keys.derived_aes128_ctr_blake3_hmac_keys_bs58,
            )
            .unwrap();
            // TODO: SECURITY:
            // this is actually what we have been doing in the past, however,
            // after looking deeper into implementation it seems that only checks the encryption
            // key part of the shared keys. the MAC key might still be wrong
            // (though I don't see how could this happen unless client messed with himself
            // and I don't think it could lead to any attacks, but somebody smarter should take a look)
            Ok((
                encrypted_address.verify(&client_address, &keys, &iv),
                Some(keys),
            ))
        } else {
            Ok((false, None))
        }
    }

    /// A tiny helper function to log any errors that shouldn't really have occurred when sending responses;
    fn send_handler_response(
        res_channel: ClientsHandlerResponseSender,
        response: ClientsHandlerResponse,
    ) {
        if res_channel.send(response).is_err() {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    async fn handle_authenticate_request(
        &mut self,
        address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        iv: IV,
        comm_channel: MixMessageSender,
        res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing authenticate client request: {:?}",
            address.as_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!("Tried to process authenticate request for a client with an already opened connection!");
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        match self
            .verify_stored_shared_key(address, encrypted_address, iv)
            .await
        {
            Err(err) => {
                error!("We failed to read client's stored shared key... - {}", err);
                self.send_error_response("Internal gateway storage error", res_channel);
                return;
            }
            Ok((false, _)) => {
                Self::send_handler_response(res_channel, ClientsHandlerResponse::Authenticate(None))
            }
            Ok((true, shared_keys)) => {
                self.push_stored_messages_to_client_and_save_channel(address, comm_channel)
                    .await;
                Self::send_handler_response(
                    res_channel,
                    ClientsHandlerResponse::Authenticate(shared_keys),
                )
            }
        }
    }

    fn handle_disconnect(&mut self, address: DestinationAddressBytes) {
        debug!(
            "Processing disconnect client request: {:?}",
            address.as_base58_string()
        );
        self.open_connections.remove(&address);
    }

    fn handle_is_online_request(
        &self,
        address: DestinationAddressBytes,
        res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing is online request for: {:?}",
            address.as_base58_string()
        );

        let response_value = self.open_connections.get(&address).cloned();
        // if this fails, it's a critical failure, because mix handlers should ALWAYS be online
        res_channel
            .send(ClientsHandlerResponse::IsOnline(response_value))
            .unwrap();
    }

    pub(crate) async fn run(
        &mut self,
        mut request_receiver_channel: ClientsHandlerRequestReceiver,
    ) {
        while let Some(request) = request_receiver_channel.next().await {
            match request {
                ClientsHandlerRequest::Register(
                    address,
                    derived_shared_key,
                    comm_channel,
                    res_channel,
                ) => {
                    self.handle_register_request(
                        address,
                        derived_shared_key,
                        comm_channel,
                        res_channel,
                    )
                    .await
                }
                ClientsHandlerRequest::Authenticate(
                    address,
                    encrypted_address,
                    iv,
                    comm_channel,
                    res_channel,
                ) => {
                    self.handle_authenticate_request(
                        address,
                        encrypted_address,
                        iv,
                        comm_channel,
                        res_channel,
                    )
                    .await
                }
                ClientsHandlerRequest::Disconnect(address) => self.handle_disconnect(address),
                ClientsHandlerRequest::IsOnline(address, res_channel) => {
                    self.handle_is_online_request(address, res_channel)
                }
            };
        }
        error!("Something bad has happened and we stopped listening for requests!");
    }

    pub(crate) fn start(mut self) -> (JoinHandle<()>, ClientsHandlerRequestSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            tokio::spawn(async move { self.run(receiver).await }),
            sender,
        )
    }
}
