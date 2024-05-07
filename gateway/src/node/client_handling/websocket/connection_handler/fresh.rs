// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::RequestHandlingError;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use log::*;
use nym_crypto::asymmetric::identity;
use nym_gateway_requests::authentication::encrypted_address::{
    EncryptedAddressBytes, EncryptedAddressConversionError,
};
use nym_gateway_requests::registration::handshake::shared_key::SharedKeyConversionError;
use nym_gateway_requests::{
    iv::{IVConversionError, IV},
    registration::handshake::{error::HandshakeError, gateway_handshake, SharedKeys},
    types::{ClientControlRequest, ServerResponse},
    BinaryResponse, CURRENT_PROTOCOL_VERSION, INITIAL_PROTOCOL_VERSION,
};
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_sphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::AvailableBandwidth;
use crate::node::{
    client_handling::{
        active_clients::ActiveClientsStore,
        websocket::{
            connection_handler::{
                AuthenticatedHandler, ClientDetails, InitialAuthResult, SocketStream,
            },
            message_receiver::{IsActive, IsActiveRequestSender},
        },
    },
    storage::{error::StorageError, Storage},
};

#[derive(Debug, Error)]
pub(crate) enum InitialAuthenticationError {
    #[error("Internal gateway storage error")]
    StorageError(#[from] StorageError),

    #[error(
        "our datastore is corrupted. the stored key for client {client_id} is malformed: {source}"
    )]
    MalformedStoredSharedKey {
        client_id: String,
        #[source]
        source: SharedKeyConversionError,
    },

    #[error("Failed to perform registration handshake: {0}")]
    HandshakeError(#[from] HandshakeError),

    #[error("Provided client address is malformed: {0}")]
    // sphinx error is not used here directly as its messaging might be confusing to people
    MalformedClientAddress(String),

    #[error("Provided encrypted client address is malformed: {0}")]
    MalformedEncryptedAddress(#[from] EncryptedAddressConversionError),

    #[error("There is already an open connection to this client")]
    DuplicateConnection,

    #[error("Provided authentication IV is malformed: {0}")]
    MalformedIV(#[from] IVConversionError),

    #[error("Only 'Register' or 'Authenticate' requests are allowed")]
    InvalidRequest,

    #[error("Experienced connection error: {0}")]
    ConnectionError(#[from] WsError),

    #[error("Attempted to negotiate connection with client using incompatible protocol version. Ours is {current} and the client reports {client:?}")]
    IncompatibleProtocol { client: Option<u8>, current: u8 },

    #[error("failed to send authentication error response: {source}")]
    ErrorResponseSendFailure {
        #[source]
        source: WsError,
    },

    #[error("failed to send authentication response: {source}")]
    ResponseSendFailure {
        #[source]
        source: WsError,
    },

    #[error("possibly received a sphinx packet without prior authentication. Request is going to be ignored")]
    BinaryRequestWithoutAuthentication,

    #[error("received a connection close message")]
    CloseMessage,

    #[error("the connection has unexpectedly closed")]
    ClosedConnection,

    #[error("failed to obtain message from websocket stream: {source}")]
    FailedToReadMessage {
        #[source]
        source: WsError,
    },

    #[error("could not establish client details")]
    EmptyClientDetails,

    #[error("failed to upgrade the client handler: {source}")]
    HandlerUpgradeFailure { source: RequestHandlingError },
}

impl InitialAuthenticationError {
    /// Converts this Error into an appropriate websocket Message.
    fn to_error_message(&self) -> Message {
        ServerResponse::new_error(self.to_string()).into()
    }
}

pub(crate) struct FreshHandler<R, S, St> {
    rng: R,
    pub(crate) shared_state: CommonHandlerState,
    pub(crate) active_clients_store: ActiveClientsStore,
    pub(crate) outbound_mix_sender: MixForwardingSender,
    pub(crate) socket_connection: SocketStream<S>,
    pub(crate) storage: St,

    // currently unused (but populated)
    pub(crate) negotiated_protocol: Option<u8>,
}

impl<R, S, St> FreshHandler<R, S, St>
where
    R: Rng + CryptoRng,
    St: Storage,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    // also at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        rng: R,
        conn: S,
        outbound_mix_sender: MixForwardingSender,
        storage: St,
        active_clients_store: ActiveClientsStore,
        shared_state: CommonHandlerState,
    ) -> Self {
        FreshHandler {
            rng,
            active_clients_store,
            outbound_mix_sender,
            socket_connection: SocketStream::RawTcp(conn),
            storage,
            negotiated_protocol: None,
            shared_state,
        }
    }

    /// Attempts to perform websocket handshake with the remote and upgrades the raw TCP socket
    /// to the framed WebSocket.
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

    /// Using received `init_msg` tries to continue the registration handshake with the connected
    /// client to establish shared keys.
    ///
    /// # Arguments
    ///
    /// * `init_msg`: a client handshake init message which should contain its identity public key as well as an ephemeral key.
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
                    self.shared_state.local_identity.as_ref(),
                    init_msg,
                )
                .await
            }
            _ => unreachable!(),
        }
    }

    /// Attempts to read websocket message from the associated socket.
    pub(crate) async fn read_websocket_message(&mut self) -> Option<Result<Message, WsError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    /// Attempts to write a single websocket message on the available socket.
    ///
    /// # Arguments
    ///
    /// * `msg`: WebSocket message to write back to the client.
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

    /// Sends unwrapped sphinx packets (payloads) back to the client. Note that each message is encrypted and tagged with
    /// the previously derived shared keys.
    ///
    /// # Arguments
    ///
    /// * `shared_keys`: keys derived between the client and gateway.
    /// * `packets`: unwrapped packets that are to be pushed back to the client.
    pub(crate) async fn push_packets_to_client(
        &mut self,
        shared_keys: &SharedKeys,
        packets: Vec<Vec<u8>>,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let messages: Vec<Result<Message, WsError>> = packets
            .into_iter()
            .map(|received_message| {
                Ok(BinaryResponse::new_pushed_mix_message(received_message)
                    .into_ws_message(shared_keys))
            })
            .collect();
        let mut send_stream = futures::stream::iter(messages);
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => {
                ws_stream.send_all(&mut send_stream).await
            }
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    /// Attempts to extract clients identity key from the received registration handshake init message.
    ///
    /// # Arguments
    ///
    /// * `init_data`: received init message that should contain, among other things, client's public key.
    // Note: this is out of the scope of this PR, but in the future, this should be removed in favour
    // of doing full parse of the init_data elsewhere
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

    /// Attempts to retrieve all messages currently stored in the persistent database to the client,
    /// which was offline at the time of their receipt.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client that is going to receive the messages.
    /// * `shared_keys`: shared keys derived between the client and the gateway used to encrypt and tag the messages.
    async fn push_stored_messages_to_client(
        &mut self,
        client_address: DestinationAddressBytes,
        shared_keys: &SharedKeys,
    ) -> Result<(), InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let mut start_next_after = None;
        loop {
            // retrieve some messages
            let (messages, new_start_next_after) = self
                .storage
                .retrieve_messages(client_address, start_next_after)
                .await?;

            let (messages, ids) = messages
                .into_iter()
                .map(|msg| (msg.content, msg.id))
                .unzip();

            // push them to the client
            if let Err(err) = self.push_packets_to_client(shared_keys, messages).await {
                warn!("We failed to send stored messages to fresh client - {err}",);
                return Err(InitialAuthenticationError::ConnectionError(err));
            } else {
                // if it was successful - remove them from the store
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
    /// Returns the retrieved shared keys if the check was successful.
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
            // this should never fail as we only ever construct persisted shared keys ourselves when inserting
            // data to the storage. The only way it could fail is if we somehow changed implementation without
            // performing proper migration
            let keys = SharedKeys::try_from_base58_string(
                shared_keys.derived_aes128_ctr_blake3_hmac_keys_bs58,
            )
            .map_err(|source| {
                InitialAuthenticationError::MalformedStoredSharedKey {
                    client_id: client_address.as_base58_string(),
                    source,
                }
            })?;

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

    fn negotiate_client_protocol(
        &self,
        client_protocol: Option<u8>,
    ) -> Result<u8, InitialAuthenticationError> {
        debug!("client protocol: {client_protocol:?}, ours: {CURRENT_PROTOCOL_VERSION}");
        let Some(client_protocol_version) = client_protocol else {
            warn!("the client we're connected to has not specified its protocol version. It's probably running version < 1.1.X, but that's still fine for now. It will become a hard error in 1.2.0");
            // note: in +1.2.0 we will have to return a hard error here
            return Ok(INITIAL_PROTOCOL_VERSION);
        };

        // a v2 gateway will understand v1 requests, but v1 client will not understand v2 responses
        if client_protocol_version == 1 {
            return Ok(1);
        }

        // we can't handle clients with higher protocol than ours
        // (perhaps we could try to negotiate downgrade on our end? sounds like a nice future improvement)
        if client_protocol_version <= CURRENT_PROTOCOL_VERSION {
            info!("the client is using exactly the same (or older) protocol version as we are. We're good to continue!");
            Ok(CURRENT_PROTOCOL_VERSION)
        } else {
            let err = InitialAuthenticationError::IncompatibleProtocol {
                client: client_protocol,
                current: CURRENT_PROTOCOL_VERSION,
            };
            error!("{err}");
            Err(err)
        }
    }

    /// Using the received challenge data, i.e. client's address as well the ciphertext of it plus
    /// a fresh IV, attempts to authenticate the client by checking whether the ciphertext matches
    /// the expected value if encrypted with the shared key.
    ///
    /// Finally, upon completion, all previously stored messages are pushed back to the client.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client wishing to authenticate.
    /// * `encrypted_address`: ciphertext of the address of the client wishing to authenticate.
    /// * `iv`: fresh IV received with the request.
    async fn authenticate_client(
        &mut self,
        client_address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        iv: IV,
    ) -> Result<Option<SharedKeys>, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        debug!(
            "Processing authenticate client request for: {}",
            client_address.as_base58_string()
        );

        let shared_keys = self
            .verify_stored_shared_key(client_address, encrypted_address, iv)
            .await?;

        if let Some(shared_keys) = shared_keys {
            self.push_stored_messages_to_client(client_address, &shared_keys)
                .await?;
            Ok(Some(shared_keys))
        } else {
            Ok(None)
        }
    }

    async fn handle_duplicate_client(
        &mut self,
        address: DestinationAddressBytes,
        mut is_active_request_tx: IsActiveRequestSender,
    ) -> Result<(), InitialAuthenticationError> {
        // Ask the other connection to ping if they are still active.
        // Use a oneshot channel to return the result to us
        let (ping_result_sender, ping_result_receiver) = oneshot::channel();
        debug!("Asking other connection handler to ping the connected client to see if it is still active");
        if let Err(err) = is_active_request_tx.send(ping_result_sender).await {
            warn!("Failed to send ping request to other handler: {err}");
        }

        // Wait for the reply
        match tokio::time::timeout(Duration::from_millis(2000), ping_result_receiver).await {
            Ok(Ok(res)) => {
                match res {
                    IsActive::NotActive => {
                        // The other handler reported that the client is not active, so we can
                        // disconnect the other client and continue with this connection.
                        debug!("Other handler reports it is not active");
                        self.active_clients_store.disconnect(address);
                    }
                    IsActive::Active => {
                        // The other handled reported a positive reply, so we have to assume it's
                        // still active and disconnect this connection.
                        info!("Other handler reports it is active");
                        return Err(InitialAuthenticationError::DuplicateConnection);
                    }
                    IsActive::BusyPinging => {
                        // The other handler is already busy pinging the client, so we have to
                        // assume it's still active and disconnect this connection.
                        debug!("Other handler reports it is already busy pinging the client");
                        return Err(InitialAuthenticationError::DuplicateConnection);
                    }
                }
            }
            Ok(Err(_)) => {
                // Other channel failed to reply (the channel sender probably dropped)
                info!("Other connection failed to reply, disconnecting it in favour of this new connection");
                self.active_clients_store.disconnect(address);
            }
            Err(_) => {
                // Timeout waiting for reply
                warn!(
                    "Other connection timed out, disconnecting it in favour of this new connection"
                );
                self.active_clients_store.disconnect(address);
            }
        }
        Ok(())
    }

    /// Tries to handle the received authentication request by checking correctness of the received data.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client wishing to authenticate.
    /// * `encrypted_address`: ciphertext of the address of the client wishing to authenticate.
    /// * `iv`: fresh IV received with the request.
    async fn handle_authenticate(
        &mut self,
        client_protocol_version: Option<u8>,
        address: String,
        enc_address: String,
        iv: String,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let negotiated_protocol = self.negotiate_client_protocol(client_protocol_version)?;
        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(negotiated_protocol);

        let address = DestinationAddressBytes::try_from_base58_string(address)
            .map_err(|err| InitialAuthenticationError::MalformedClientAddress(err.to_string()))?;
        let encrypted_address = EncryptedAddressBytes::try_from_base58_string(enc_address)?;
        let iv = IV::try_from_base58_string(iv)?;

        // Check for duplicate clients
        if let Some(client_tx) = self.active_clients_store.get_remote_client(address) {
            warn!("Detected duplicate connection for client: {address}");
            self.handle_duplicate_client(address, client_tx.is_active_request_sender)
                .await?;
        }

        let shared_keys = self
            .authenticate_client(address, encrypted_address, iv)
            .await?;
        let status = shared_keys.is_some();

        let available_bandwidth: AvailableBandwidth =
            self.storage.get_available_bandwidth(address).await?.into();

        let bandwidth_remaining = if available_bandwidth.expired() {
            self.expire_bandwidth(address).await?;
            0
        } else {
            available_bandwidth.bytes
        };

        let client_details =
            shared_keys.map(|shared_keys| ClientDetails::new(address, shared_keys));

        Ok(InitialAuthResult::new(
            client_details,
            ServerResponse::Authenticate {
                protocol_version: Some(negotiated_protocol),
                status,
                bandwidth_remaining,
            },
        ))
    }

    pub(crate) async fn expire_bandwidth(
        &self,
        client: DestinationAddressBytes,
    ) -> Result<(), StorageError> {
        self.storage.reset_bandwidth(client).await
    }

    /// Attempts to finalize registration of the client by storing the derived shared keys in the
    /// persistent store as well as creating entry for its bandwidth allocation.
    ///
    /// Finally, upon completion, all previously stored messages are pushed back to the client.
    ///
    /// # Arguments
    ///
    /// * `client`: details (i.e. address and shared keys) of the registered client
    async fn register_client(
        &mut self,
        client: &ClientDetails,
    ) -> Result<bool, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        debug!(
            "Processing register client request for: {}",
            client.address.as_base58_string()
        );

        self.storage
            .insert_shared_keys(client.address, &client.shared_keys)
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

        self.push_stored_messages_to_client(client.address, &client.shared_keys)
            .await?;

        Ok(true)
    }

    /// Tries to handle the received register request by checking attempting to complete registration
    /// handshake using the received data.
    ///
    /// # Arguments
    ///
    /// * `init_data`: init payload of the registration handshake.
    async fn handle_register(
        &mut self,
        client_protocol_version: Option<u8>,
        init_data: Vec<u8>,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let negotiated_protocol = self.negotiate_client_protocol(client_protocol_version)?;
        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(negotiated_protocol);

        let remote_identity = Self::extract_remote_identity_from_register_init(&init_data)?;
        let remote_address = remote_identity.derive_destination_address();

        if self.active_clients_store.is_active(remote_address) {
            return Err(InitialAuthenticationError::DuplicateConnection);
        }

        let shared_keys = self.perform_registration_handshake(init_data).await?;
        let client_details = ClientDetails::new(remote_address, shared_keys);

        let status = self.register_client(&client_details).await?;

        Ok(InitialAuthResult::new(
            Some(client_details),
            ServerResponse::Register {
                protocol_version: Some(negotiated_protocol),
                status,
            },
        ))
    }

    /// Handles data that resembles request to either start registration handshake or perform
    /// authentication.
    ///
    /// # Arguments
    ///
    /// * `raw_request`: raw text request received from the websocket.
    async fn handle_initial_authentication_request(
        &mut self,
        raw_request: String,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Ok(request) = ClientControlRequest::try_from(raw_request) {
            match request {
                ClientControlRequest::Authenticate {
                    protocol_version,
                    address,
                    enc_address,
                    iv,
                } => {
                    self.handle_authenticate(protocol_version, address, enc_address, iv)
                        .await
                }
                ClientControlRequest::RegisterHandshakeInitRequest {
                    protocol_version,
                    data,
                } => self.handle_register(protocol_version, data).await,
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
    ) -> Result<AuthenticatedHandler<R, S, St>, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        trace!("Started waiting for authenticate/register request...");

        while let Some(msg) = self.read_websocket_message().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(source) => {
                    debug!("failed to obtain message from websocket stream! stopping connection handler: {source}");
                    return Err(InitialAuthenticationError::FailedToReadMessage { source });
                }
            };

            if msg.is_close() {
                return Err(InitialAuthenticationError::CloseMessage);
            }

            // ONLY handle 'Authenticate' or 'Register' requests, ignore everything else
            match msg {
                // we have explicitly checked for close message
                Message::Close(_) => unreachable!(),
                Message::Text(text_msg) => {
                    let (mix_sender, mix_receiver) = mpsc::unbounded();
                    return match self.handle_initial_authentication_request(text_msg).await {
                        Err(err) => {
                            debug!("authentication failure: {err}");

                            // try to send error to the client
                            if let Err(source) =
                                self.send_websocket_message(err.to_error_message()).await
                            {
                                debug!("Failed to send authentication error response: {source}");
                                return Err(InitialAuthenticationError::ErrorResponseSendFailure {
                                    source,
                                });
                            }
                            // return the underlying error
                            Err(err)
                        }
                        Ok(auth_result) => {
                            // try to send auth response back to the client
                            if let Err(source) = self
                                .send_websocket_message(auth_result.server_response.into())
                                .await
                            {
                                debug!("Failed to send authentication response: {source}");
                                return Err(InitialAuthenticationError::ResponseSendFailure {
                                    source,
                                });
                            }

                            if let Some(client_details) = auth_result.client_details {
                                // Channel for handlers to ask other handlers if they are still active.
                                let (is_active_request_sender, is_active_request_receiver) =
                                    mpsc::unbounded();
                                self.active_clients_store.insert_remote(
                                    client_details.address,
                                    mix_sender,
                                    is_active_request_sender,
                                );
                                AuthenticatedHandler::upgrade(
                                    self,
                                    client_details,
                                    mix_receiver,
                                    is_active_request_receiver,
                                )
                                .await
                                .map_err(|source| {
                                    InitialAuthenticationError::HandlerUpgradeFailure { source }
                                })
                            } else {
                                // honestly, it's been so long I don't remember under what conditions its possible (if at all)
                                // to have empty client details
                                Err(InitialAuthenticationError::EmptyClientDetails)
                            }
                        }
                    };
                }
                Message::Binary(_) => {
                    // perhaps logging level should be reduced here, let's leave it for now and see what happens
                    // if client is working correctly, this should have never happened
                    debug!("possibly received a sphinx packet without prior authentication. Request is going to be ignored");
                    if let Err(source) = self
                        .send_websocket_message(
                            ServerResponse::new_error(
                                "binary request without prior authentication",
                            )
                            .into(),
                        )
                        .await
                    {
                        return Err(InitialAuthenticationError::ErrorResponseSendFailure {
                            source,
                        });
                    }
                    return Err(InitialAuthenticationError::BinaryRequestWithoutAuthentication);
                }

                _ => continue,
            };
        }

        Err(InitialAuthenticationError::ClosedConnection)
    }

    pub(crate) async fn start_handling(self, shutdown: nym_task::TaskClient)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        super::handle_connection(self, shutdown).await
    }
}
