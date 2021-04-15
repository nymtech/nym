// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::{
    client_handling::websocket::message_receiver::MixMessageSender,
    storage::{inboxes::ClientStorage, ClientLedger},
};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::authentication::iv::AuthenticationIV;
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
        AuthenticationIV,
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
    clients_ledger: ClientLedger,
    clients_inbox_storage: ClientStorage,
}

impl ClientsHandler {
    pub(crate) fn new(clients_ledger: ClientLedger, clients_inbox_storage: ClientStorage) -> Self {
        ClientsHandler {
            open_connections: HashMap::new(),
            clients_ledger,
            clients_inbox_storage,
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
        // And only after this method exists, mix receivers will become aware of the client
        // connection going online and being able to forward traffic there.
        //
        // possible solution: spawn a future to empty inbox in X seconds rather than immediately
        // JS: I will most likely do that (with including entries to config, etc.) once the
        // basic version is up and running as not to waste time on it now

        // NOTE: THIS IGNORES MESSAGE RETRIEVAL LIMIT AND TAKES EVERYTHING!
        let all_stored_messages = match self
            .clients_inbox_storage
            .retrieve_all_client_messages(client_address)
            .await
        {
            Ok(msgs) => msgs,
            Err(e) => {
                error!(
                    "failed to retrieve client messages. {:?} inbox might be corrupted now - {:?}",
                    client_address.to_base58_string(),
                    e
                );
                return;
            }
        };

        let (messages, paths): (Vec<_>, Vec<_>) = all_stored_messages
            .into_iter()
            .map(|c| c.into_tuple())
            .unzip();

        if comm_channel.unbounded_send(messages).is_err() {
            error!("Somehow we failed to stored messages to a fresh client channel - there seem to be a weird bug present!");
        } else {
            // but if all went well, we can now delete it
            if let Err(e) = self.clients_inbox_storage.delete_files(paths).await {
                error!(
                    "Failed to remove client ({:?}) files - {:?}",
                    client_address.to_base58_string(),
                    e
                );
            } else {
                // finally, everything was fine - we retrieved everything, we deleted everything,
                // we assume we can now safely delegate client message pushing
                self.open_connections.insert(client_address, comm_channel);
            }
        }
    }

    async fn handle_register_request(
        &mut self,
        address: DestinationAddressBytes,
        derived_shared_key: SharedKeys,
        comm_channel: MixMessageSender,
        res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing register new client request: {:?}",
            address.to_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!(
                "Tried to process register request for a client with an already opened connection!"
            );
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        if self
            .clients_ledger
            .insert_shared_key(derived_shared_key, address)
            .unwrap()
            .is_some()
        {
            info!(
                "Client {:?} was already registered before!",
                address.to_base58_string()
            )
        } else if let Err(e) = self.clients_inbox_storage.create_storage_dir(address).await {
            error!("We failed to create inbox directory for the client -{:?}\nReverting stored shared key...", e);
            // we must revert our changes if this operation failed
            self.clients_ledger.remove_shared_key(&address).unwrap();
            self.send_error_response("failed to complete issuing shared key", res_channel);
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

    async fn handle_authenticate_request(
        &mut self,
        address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        iv: AuthenticationIV,
        comm_channel: MixMessageSender,
        res_channel: ClientsHandlerResponseSender,
    ) {
        debug!(
            "Processing authenticate client request: {:?}",
            address.to_base58_string()
        );

        if self.open_connections.get(&address).is_some() {
            warn!("Tried to process authenticate request for a client with an already opened connection!");
            self.send_error_response("duplicate connection detected", res_channel);
            return;
        }

        if self
            .clients_ledger
            .verify_shared_key(&address, &encrypted_address, &iv)
            .unwrap()
        {
            // The first unwrap is due to possible db read errors, but I'm not entirely sure when could
            // the second one happen.
            let shared_key = self
                .clients_ledger
                .get_shared_key(&address)
                .unwrap()
                .unwrap();
            self.push_stored_messages_to_client_and_save_channel(address, comm_channel)
                .await;
            if res_channel
                .send(ClientsHandlerResponse::Authenticate(Some(shared_key)))
                .is_err()
            {
                error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
            }
        } else if res_channel
            .send(ClientsHandlerResponse::Authenticate(None))
            .is_err()
        {
            error!("Somehow we failed to send response back to websocket handler - there seem to be a weird bug present!");
        }
    }

    fn handle_disconnect(&mut self, address: DestinationAddressBytes) {
        debug!(
            "Processing disconnect client request: {:?}",
            address.to_base58_string()
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
            address.to_base58_string()
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
