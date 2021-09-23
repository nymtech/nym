// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::connection_handler::{
    AuthenticatedHandler, ClientDetails, InitialAuthResult, SocketStream,
};
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use crate::node::storage::error::StorageError;
use crate::node::storage::PersistentStorage;
use coconut_interface::VerificationKey;
use crypto::asymmetric::identity;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use gateway_requests::authentication::encrypted_address::{
    EncryptedAddressBytes, EncryptedAddressConversionError,
};
use gateway_requests::iv::{IVConversionError, IV};
use gateway_requests::registration::handshake::error::HandshakeError;
use gateway_requests::registration::handshake::{gateway_handshake, SharedKeys};
use gateway_requests::types::{ClientControlRequest, ServerResponse};
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use nymsphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

#[derive(Debug, Error)]
enum InitialAuthenticationError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] StorageError),

    #[error("Failed to perform registration handshake - {0}")]
    HandshakeError(#[from] HandshakeError),

    #[error("Provided client address is malformed - {0}")]
    // sphinx error is not used here directly as it's messaging might be confusing to people
    MalformedClientAddress(String),

    #[error("Provided encrypted client address is malformed - {0}")]
    MalformedEncryptedAddress(#[from] EncryptedAddressConversionError),

    #[error("There is already an open connection to this client")]
    DuplicateConnection,

    #[error("Provided authentication IV is malformed - {0}")]
    MalformedIV(#[from] IVConversionError),

    #[error("Only 'Register' or 'Authenticate' requests are allowed")]
    InvalidRequest,
}

impl InitialAuthenticationError {
    fn into_error_message(self) -> Message {
        ServerResponse::new_error(self.to_string()).into()
    }
}

pub(crate) struct FreshHandler<R, S> {
    rng: R,
    local_identity: Arc<identity::KeyPair>,
    pub(crate) active_clients_store: ActiveClientsStore,
    pub(crate) aggregated_verification_key: VerificationKey,
    pub(crate) outbound_mix_sender: MixForwardingSender,
    pub(crate) socket_connection: SocketStream<S>,
    pub(crate) storage: PersistentStorage,
}

impl<R, S> FreshHandler<R, S>
where
    R: Rng + CryptoRng,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(
        rng: R,
        conn: S,
        outbound_mix_sender: MixForwardingSender,
        local_identity: Arc<identity::KeyPair>,
        aggregated_verification_key: VerificationKey,
        storage: PersistentStorage,
        active_clients_store: ActiveClientsStore,
    ) -> Self {
        FreshHandler {
            rng,
            active_clients_store,
            outbound_mix_sender,
            socket_connection: SocketStream::RawTcp(conn),
            local_identity,
            aggregated_verification_key,
            storage,
        }
    }

    pub(crate) async fn perform_websocket_handshake(&mut self) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.socket_connection =
            match std::mem::replace(&mut self.socket_connection, SocketStream::Invalid) {
                SocketStream::RawTcp(conn) => {
                    // TODO: perhaps in the future, rather than panic here (and uncleanly shut tcp stream)
                    // return a result with an error?
                    let ws_stream = tokio_tungstenite::accept_async(conn).await?;
                    SocketStream::UpgradedWebSocket(ws_stream)
                }
                other => other,
            };
        Ok(())
    }

    async fn perform_registration_handshake(
        &mut self,
        init_msg: Vec<u8>,
    ) -> Result<SharedKeys, HandshakeError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        debug_assert!(self.socket_connection.is_websocket());
        match &mut self.socket_connection {
            SocketStream::UpgradedWebSocket(ws_stream) => {
                gateway_handshake(
                    &mut self.rng,
                    ws_stream,
                    self.local_identity.as_ref(),
                    init_msg,
                )
                .await
            }
            _ => unreachable!(),
        }
    }

    pub(crate) async fn read_websocket_message(&mut self) -> Option<Result<Message, WsError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    pub(crate) async fn send_websocket_message(&mut self, msg: Message) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            // TODO: more closely investigate difference between `Sink::send` and `Sink::send_all`
            // it got something to do with batching and flushing - it might be important if it
            // turns out somehow we've got a bottleneck here
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.send(msg).await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    fn extract_remote_identity_from_register_init(
        init_data: &[u8],
    ) -> Result<identity::PublicKey, InitialAuthenticationError> {
        if init_data.len() < identity::PUBLIC_KEY_LENGTH {
            Err(InitialAuthenticationError::HandshakeError(
                HandshakeError::MalformedRequest,
            ))
        } else {
            identity::PublicKey::from_bytes(&init_data[..identity::PUBLIC_KEY_LENGTH]).map_err(
                |_| InitialAuthenticationError::HandshakeError(HandshakeError::MalformedRequest),
            )
        }
    }

    async fn push_stored_messages_to_client(
        &self,
        client_address: DestinationAddressBytes,
        comm_channel: &MixMessageSender,
    ) -> Result<(), InitialAuthenticationError> {
        // TODO: SECURITY (kinda):
        // We should stagger reading the messages in a different way, i.e. we read some of them,
        // send them all the way back to the client and then read next batch. Otherwise we risk
        // being vulnerable to trivial attacks causing gateway crashes.
        let mut start_next_after = None;
        loop {
            let (messages, new_start_next_after) = self
                .storage
                .retrieve_messages(client_address, start_next_after)
                .await?;

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
                self.storage.remove_messages(ids).await?;
            }

            // no more messages to grab
            if new_start_next_after.is_none() {
                break;
            } else {
                start_next_after = new_start_next_after
            }
        }

        Ok(())
    }

    /// Checks whether the stored shared keys match the received data, i.e. whether the upon decryption
    /// the provided encrypted address matches the expected unencrypted address.
    ///
    /// Returns the the retrieved shared keys if the check was successful.
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
    ) -> Result<Option<SharedKeys>, InitialAuthenticationError> {
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
            if encrypted_address.verify(&client_address, &keys, &iv) {
                Ok(Some(keys))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn authenticate_client(
        &self,
        client_address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        iv: IV,
        sender_channel: MixMessageSender,
    ) -> Result<Option<SharedKeys>, InitialAuthenticationError> {
        debug!(
            "Processing authenticate client request for: {}",
            client_address.as_base58_string()
        );

        let shared_keys = self
            .verify_stored_shared_key(client_address, encrypted_address, iv)
            .await?;

        if let Some(shared_keys) = shared_keys {
            self.push_stored_messages_to_client(client_address, &sender_channel)
                .await?;
            self.active_clients_store
                .insert(client_address, sender_channel);

            Ok(Some(shared_keys))
        } else {
            Ok(None)
        }
    }

    async fn handle_authenticate(
        &mut self,
        address: String,
        enc_address: String,
        iv: String,
        mix_sender: MixMessageSender,
    ) -> Result<InitialAuthResult, InitialAuthenticationError> {
        let address = DestinationAddressBytes::try_from_base58_string(address)
            .map_err(|err| InitialAuthenticationError::MalformedClientAddress(err.to_string()))?;
        let encrypted_address = EncryptedAddressBytes::try_from_base58_string(enc_address)?;
        let iv = IV::try_from_base58_string(iv)?;

        if self.active_clients_store.get(address).is_some() {
            return Err(InitialAuthenticationError::DuplicateConnection);
        }

        let shared_keys = self
            .authenticate_client(address, encrypted_address, iv, mix_sender)
            .await?;
        let status = shared_keys.is_some();
        let client_details =
            shared_keys.map(|shared_keys| ClientDetails::new(address, shared_keys));

        Ok(InitialAuthResult::new(
            client_details,
            ServerResponse::Authenticate { status },
        ))
    }

    async fn register_client(
        &self,
        client: ClientDetails,
        sender_channel: MixMessageSender,
    ) -> Result<bool, InitialAuthenticationError> {
        debug!(
            "Processing register client request for: {}",
            client.address.as_base58_string()
        );

        self.storage
            .insert_shared_keys(client.address, client.shared_keys)
            .await?;

        // see if we have bandwidth entry for the client already, if not, create one with zero value
        if self
            .storage
            .get_available_bandwidth(client.address)
            .await?
            .is_none()
        {
            self.storage.create_bandwidth_entry(client.address).await?;
        }

        self.push_stored_messages_to_client(client.address, &sender_channel)
            .await?;

        self.active_clients_store
            .insert(client.address, sender_channel);
        Ok(true)
    }

    async fn handle_register(
        &mut self,
        init_data: Vec<u8>,
        mix_sender: MixMessageSender,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let remote_identity = Self::extract_remote_identity_from_register_init(&init_data)?;
        let remote_address = remote_identity.derive_destination_address();

        if self.active_clients_store.get(remote_address).is_some() {
            return Err(InitialAuthenticationError::DuplicateConnection);
        }

        let shared_keys = self.perform_registration_handshake(init_data).await?;
        let client_details = ClientDetails::new(remote_address, shared_keys);

        let status = self.register_client(client_details, mix_sender).await?;

        Ok(InitialAuthResult::new(
            Some(client_details),
            ServerResponse::Register { status },
        ))
    }

    /// Handles data that resembles request to either start registration handshake or perform
    /// authentication.
    async fn handle_initial_authentication_request(
        &mut self,
        mix_sender: MixMessageSender,
        raw_request: String,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Ok(request) = ClientControlRequest::try_from(raw_request) {
            match request {
                ClientControlRequest::Authenticate {
                    address,
                    enc_address,
                    iv,
                } => {
                    self.handle_authenticate(address, enc_address, iv, mix_sender)
                        .await
                }
                ClientControlRequest::RegisterHandshakeInitRequest { data } => {
                    self.handle_register(data, mix_sender).await
                }
                // won't accept anything else (like bandwidth) without prior authentication
                _ => Err(InitialAuthenticationError::InvalidRequest),
            }
        } else {
            Err(InitialAuthenticationError::InvalidRequest)
        }
    }

    /// Listens for only a subset of possible client requests, i.e. for those that can either
    /// result in client getting registered or authenticated. All other requests, such as forwarding
    /// sphinx packets considered an error and terminate the connection.
    // TODO: somehow cleanup this method
    pub(crate) async fn perform_initial_authentication(
        mut self,
    ) -> Option<AuthenticatedHandler<R, S>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        trace!("Started waiting for authenticate/register request...");

        while let Some(msg) = self.read_websocket_message().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(err) => {
                    error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                    break;
                }
            };

            if msg.is_close() {
                break;
            }

            // ONLY handle 'Authenticate' or 'Register' requests, ignore everything else
            match msg {
                Message::Close(_) => break,
                Message::Text(text_msg) => {
                    let (mix_sender, mix_receiver) = mpsc::unbounded();
                    match self
                        .handle_initial_authentication_request(mix_sender, text_msg)
                        .await
                    {
                        Err(err) => {
                            if let Err(err) =
                                self.send_websocket_message(err.into_error_message()).await
                            {
                                debug!("Failed to send authentication error response - {}", err);
                                return None;
                            }
                        }
                        Ok(auth_result) => {
                            if let Err(err) = self
                                .send_websocket_message(auth_result.server_response.into())
                                .await
                            {
                                debug!("Failed to send authentication response - {}", err);
                                return None;
                            }

                            return auth_result.client_details.map(|client_details| {
                                AuthenticatedHandler::upgrade(self, client_details, mix_receiver)
                            });
                        }
                    }
                }
                Message::Binary(_) => {
                    // perhaps logging level should be reduced here, let's leave it for now and see what happens
                    // if client is working correctly, this should have never happened
                    warn!("possibly received a sphinx packet without prior authentication. Request is going to be ignored");
                    if let Err(err) = self
                        .send_websocket_message(
                            ServerResponse::new_error(
                                "binary request without prior authentication",
                            )
                            .into(),
                        )
                        .await
                    {
                        debug!(
                            "Failed to send error response during authentication - {}",
                            err
                        )
                    }
                    return None;
                }

                _ => continue,
            };
        }
        None
    }

    pub(crate) async fn start_handling(self)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        super::handle_connection(self).await
    }
}
