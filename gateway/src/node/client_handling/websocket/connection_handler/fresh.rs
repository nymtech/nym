// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::active_clients::RemoteClientData;
use crate::node::client_handling::websocket::common_state::CommonHandlerState;
use crate::node::client_handling::websocket::connection_handler::helpers::KeyWithAuthTimestamp;
use crate::node::client_handling::websocket::connection_handler::INITIAL_MESSAGE_TIMEOUT;
use crate::node::client_handling::websocket::{
    connection_handler::{AuthenticatedHandler, ClientDetails, InitialAuthResult, SocketStream},
    message_receiver::IsActive,
};
use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use nym_bin_common::opentelemetry::context::{new_span_context_with_id, ManualSpanContextExt};
use nym_credentials_interface::AvailableBandwidth;
use nym_crypto::aes::cipher::crypto_common::rand_core::RngCore;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_requests::authenticate::AuthenticateRequest;
use nym_gateway_requests::authentication::encrypted_address::{
    EncryptedAddressBytes, EncryptedAddressConversionError,
};
use nym_gateway_requests::{
    registration::handshake::{error::HandshakeError, gateway_handshake},
    types::{ClientControlRequest, ServerResponse},
    AuthenticationFailure, BinaryResponse, SharedGatewayKey, CURRENT_PROTOCOL_VERSION,
    INITIAL_PROTOCOL_VERSION,
};
use nym_gateway_storage::error::GatewayStorageError;
use nym_gateway_storage::models::Client;
use nym_gateway_storage::traits::BandwidthGatewayStorage;
use nym_gateway_storage::traits::InboxGatewayStorage;
use nym_gateway_storage::traits::SharedKeyGatewayStorage;
use nym_node_metrics::events::MetricsEvent;
use nym_sphinx::DestinationAddressBytes;
use nym_task::ShutdownToken;
use opentelemetry::trace::{SpanContext, TraceContextExt, TraceFlags};
use opentelemetry::TraceId;
use opentelemetry::Context;
use opentelemetry_sdk::logs::TraceContext;
use opentelemetry_sdk::trace::{IdGenerator, RandomIdGenerator};
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use rand::CryptoRng;
use std::net::SocketAddr;
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};
use tracing::{debug, error, info, info_span, instrument, span, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;



#[derive(Debug, Error)]
pub(crate) enum InitialAuthenticationError {
    #[error(transparent)]
    AuthenticationFailure(#[from] AuthenticationFailure),

    #[error("attempted to overwrite client session with a stale authentication")]
    StaleSessionOverwrite,

    #[error("Internal gateway storage error")]
    StorageError(#[from] GatewayStorageError),

    #[error(
        "our datastore is corrupted. the stored key for client {client_id} is malformed: {source}"
    )]
    MalformedStoredSharedKey {
        client_id: String,
        #[source]
        source: GatewayStorageError,
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

    #[error("provided authentication IV is malformed: {0}")]
    MalformedIV(bs58::decode::Error),

    #[error("Only 'Register' or 'Authenticate' requests are allowed")]
    InvalidRequest,

    #[error("received a Message of type {typ} which was not expected in this context")]
    UnexpectedMessageType { typ: String },

    #[error("Experienced connection error: {0}")]
    ConnectionError(Box<WsError>),

    #[error("Attempted to negotiate connection with client using incompatible protocol version. Ours is {current} and the client reports {client:?}")]
    IncompatibleProtocol { client: Option<u8>, current: u8 },

    #[error("failed to send authentication response: {source}")]
    ResponseSendFailure {
        #[source]
        source: Box<WsError>,
    },

    #[error("possibly received a sphinx packet without prior authentication. Request is going to be ignored")]
    BinaryRequestWithoutAuthentication,

    #[error("the connection has unexpectedly closed")]
    ClosedConnection,

    #[error("failed to obtain message from websocket stream: {source}")]
    FailedToReadMessage {
        #[source]
        source: Box<WsError>,
    },

    #[error("timed out while waiting for initial data")]
    Timeout,

    #[error("could not establish client details")]
    EmptyClientDetails,
}

impl From<WsError> for InitialAuthenticationError {
    fn from(error: WsError) -> Self {
        InitialAuthenticationError::ConnectionError(Box::new(error))
    }
}

pub(crate) struct FreshHandler<R, S> {
    rng: R,
    pub(crate) shared_state: CommonHandlerState,
    pub(crate) socket_connection: SocketStream<S>,
    pub(crate) peer_address: SocketAddr,
    pub(crate) shutdown: ShutdownToken,

    // currently unused (but populated)
    pub(crate) negotiated_protocol: Option<u8>,
}

impl<R, S> FreshHandler<R, S> {
    pub(crate) fn shared_state(&self) -> &CommonHandlerState {
        &self.shared_state
    }

    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(
        rng: R,
        conn: S,
        shared_state: CommonHandlerState,
        peer_address: SocketAddr,
        shutdown: ShutdownToken,
    ) -> Self {
        FreshHandler {
            rng,
            socket_connection: SocketStream::RawTcp(conn),
            peer_address,
            negotiated_protocol: None,
            shared_state,
            shutdown,
        }
    }

    pub(crate) fn send_metrics(&self, event: impl Into<MetricsEvent>) {
        self.shared_state.metrics_sender.report_unchecked(event)
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
                    let ws_stream = Box::new(tokio_tungstenite::accept_async(conn).await?);
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
    ) -> Result<SharedGatewayKey, HandshakeError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
        R: CryptoRng + RngCore + Send,
    {
        debug_assert!(self.socket_connection.is_websocket());
        match &mut self.socket_connection {
            SocketStream::UpgradedWebSocket(ws_stream) => {
                gateway_handshake(
                    &mut self.rng,
                    ws_stream,
                    self.shared_state.local_identity.as_ref(),
                    init_msg,
                    self.shutdown.clone(),
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
    pub(crate) async fn send_websocket_message(
        &mut self,
        msg: impl Into<Message>,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            // TODO: more closely investigate difference between `Sink::send` and `Sink::send_all`
            // it got something to do with batching and flushing - it might be important if it
            // turns out somehow we've got a bottleneck here
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.send(msg.into()).await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    pub(crate) async fn send_error_response(
        &mut self,
        err: impl std::error::Error,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        self.send_websocket_message(ServerResponse::new_error(err.to_string()))
            .await
    }

    pub(crate) async fn send_and_forget_error_response(&mut self, err: impl std::error::Error)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Err(err) = self.send_error_response(err).await {
            debug!("failed to send error response: {err}")
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
        shared_keys: &SharedGatewayKey,
        packets: Vec<Vec<u8>>,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let messages: Vec<Result<Message, WsError>> = packets
            .into_iter()
            .filter_map(|message| {
                BinaryResponse::PushedMixMessage { message }
                    .into_ws_message(shared_keys)
                    .inspect_err(|err| error!("failed to encrypt client message: {err}"))
                    .ok()
            })
            .map(Ok)
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
    ) -> Result<ed25519::PublicKey, InitialAuthenticationError> {
        if init_data.len() < ed25519::PUBLIC_KEY_LENGTH {
            Err(InitialAuthenticationError::HandshakeError(
                HandshakeError::MalformedRequest,
            ))
        } else {
            ed25519::PublicKey::from_bytes(&init_data[..ed25519::PUBLIC_KEY_LENGTH]).map_err(|_| {
                InitialAuthenticationError::HandshakeError(HandshakeError::MalformedRequest)
            })
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
        shared_keys: &SharedGatewayKey,
    ) -> Result<(), InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let mut start_next_after = None;
        loop {
            // retrieve some messages
            let (messages, new_start_next_after) = self
                .shared_state
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
                return Err(InitialAuthenticationError::ConnectionError(Box::new(err)));
            } else {
                // if it was successful - remove them from the store
                self.shared_state.storage.remove_messages(ids).await?;
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

    async fn retrieve_shared_key(
        &self,
        client: DestinationAddressBytes,
    ) -> Result<Option<KeyWithAuthTimestamp>, InitialAuthenticationError> {
        let shared_keys = self.shared_state.storage.get_shared_keys(client).await?;

        let Some(stored_shared_keys) = shared_keys else {
            return Ok(None);
        };

        let keys = KeyWithAuthTimestamp::try_from_stored(stored_shared_keys, client)?;

        Ok(Some(keys))
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
    /// * `iv`: nonce/iv created for this particular encryption.
    async fn auth_v1_verify_stored_shared_key(
        &self,
        client_address: DestinationAddressBytes,
        encrypted_address: EncryptedAddressBytes,
        nonce: &[u8],
    ) -> Result<Option<KeyWithAuthTimestamp>, InitialAuthenticationError> {
        let Some(keys) = self.retrieve_shared_key(client_address).await? else {
            return Ok(None);
        };

        // LEGACY ISSUE: we're not verifying HMAC key
        if encrypted_address.verify(&client_address, &keys.key, nonce) {
            Ok(Some(keys))
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

        // #####
        // On backwards compat:
        // Currently it is the case that gateways will understand all previous protocol versions
        // and will downgrade accordingly, but this will now always be the case.
        // For example, once we remove downgrade on legacy auth, anything below version 4 will be rejected
        // #####

        // a v2 gateway will understand v1 requests, but v1 client will not understand v2 responses
        if client_protocol_version == 1 {
            return Ok(1);
        }

        // a v3 gateway will understand v2 requests (legacy keys)
        if client_protocol_version == 2 {
            return Ok(2);
        }

        // a v4 gateway will understand v3 requests (aes256gcm-siv)
        if client_protocol_version == 3 {
            return Ok(3);
        }

        // a v5 gateway will understand v4 requests (key-rotation)
        if client_protocol_version == 4 {
            return Ok(4);
        }

        // we can't handle clients with higher protocol than ours
        // (perhaps we could try to negotiate downgrade on our end? sounds like a nice future improvement)
        if client_protocol_version <= CURRENT_PROTOCOL_VERSION {
            debug!("the client is using exactly the same (or older) protocol version as we are. We're good to continue!");
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

    async fn handle_duplicate_client(
        &mut self,
        address: DestinationAddressBytes,
        remote_client_data: RemoteClientData,
        new_session_start: OffsetDateTime,
    ) -> Result<(), InitialAuthenticationError> {
        let mut is_active_request_tx = remote_client_data.channels.is_active_request_sender;

        // new session must **always** be explicitly more recent
        if new_session_start <= remote_client_data.session_request_timestamp {
            return Err(InitialAuthenticationError::StaleSessionOverwrite);
        }

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
                        self.shared_state.active_clients_store.disconnect(address);
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
                self.shared_state.active_clients_store.disconnect(address);
            }
            Err(_) => {
                // Timeout waiting for reply
                warn!(
                    "Other connection timed out, disconnecting it in favour of this new connection"
                );
                self.shared_state.active_clients_store.disconnect(address);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn get_registered_client_id(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Result<i64, InitialAuthenticationError> {
        self.shared_state
            .storage
            .get_mixnet_client_id(client_address)
            .await
            .map_err(Into::into)
    }

    async fn get_registered_available_bandwidth(
        &self,
        client_id: i64,
    ) -> Result<AvailableBandwidth, InitialAuthenticationError> {
        let available_bandwidth: AvailableBandwidth = self
            .shared_state
            .storage
            .get_available_bandwidth(client_id)
            .await?
            .map(From::from)
            .unwrap_or_default();
        Ok(available_bandwidth)
    }

    /// Tries to handle the received authentication request by checking correctness of the received data.
    ///
    /// # Arguments
    ///
    /// * `client_address`: address of the client wishing to authenticate.
    /// * `encrypted_address`: ciphertext of the address of the client wishing to authenticate.
    /// * `iv`: fresh IV received with the request.
    #[instrument(skip_all
        fields(
            address = %address,
        )
    )]
    async fn handle_legacy_authenticate(
        &mut self,
        client_protocol_version: Option<u8>,
        address: String,
        enc_address: String,
        raw_nonce: String,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        debug!("handling client authentication (v1)");

        let negotiated_protocol = self.negotiate_client_protocol(client_protocol_version)?;
        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(negotiated_protocol);

        let address = DestinationAddressBytes::try_from_base58_string(address)
            .map_err(|err| InitialAuthenticationError::MalformedClientAddress(err.to_string()))?;
        let encrypted_address = EncryptedAddressBytes::try_from_base58_string(enc_address)?;
        let nonce = bs58::decode(&raw_nonce)
            .into_vec()
            .map_err(InitialAuthenticationError::MalformedIV)?;

        // validate the shared key
        let Some(shared_keys) = self
            .auth_v1_verify_stored_shared_key(address, encrypted_address, &nonce)
            .await?
        else {
            // it feels weird to be returning an 'Ok' here, but I didn't want to change the existing behaviour
            return Ok(InitialAuthResult::new_failed(Some(negotiated_protocol)));
        };

        // in v1 we don't have explicit data so we have to use current timestamp
        // (which does nothing but just allows us to use the same codepath)
        let session_request_start = OffsetDateTime::now_utc();

        // Check for duplicate clients
        if let Some(remote_client_data) = self
            .shared_state
            .active_clients_store
            .get_remote_client(address)
        {
            warn!("Detected duplicate connection for client: {address}");
            self.handle_duplicate_client(address, remote_client_data, session_request_start)
                .await?;
        }

        let client_id = shared_keys.client_id;

        // if applicable, push stored messages
        self.push_stored_messages_to_client(address, &shared_keys.key)
            .await?;

        // check the bandwidth
        let available_bandwidth = self.get_registered_available_bandwidth(client_id).await?;

        let bandwidth_remaining = if available_bandwidth.expired() {
            self.shared_state.storage.reset_bandwidth(client_id).await?;
            0
        } else {
            available_bandwidth.bytes
        };

        Ok(InitialAuthResult::new(
            Some(ClientDetails::new(
                client_id,
                address,
                shared_keys.key,
                session_request_start,
                None,
            )),
            ServerResponse::Authenticate {
                protocol_version: Some(negotiated_protocol),
                status: true,
                bandwidth_remaining,
            },
        ))
    }

    #[instrument(skip_all, fields(
        address = %request.content.client_identity.derive_destination_address(),
    ))]
    async fn handle_authenticate_v2(
        &mut self,
        request: Box<AuthenticateRequest>,
        otel_context: Option<ManualSpanContextExt>,
    ) -> Result<InitialAuthResult, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        debug!("handling client authentication (v2)");
        tracing::info_span!("Authenticate v2");

        let negotiated_protocol =
            self.negotiate_client_protocol(Some(request.content.protocol_version))?;
        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(negotiated_protocol);

        let address = request.content.client_identity.derive_destination_address();

        // do cheap checks first
        // is the provided timestamp relatively recent (and not in the future?)
        request.verify_timestamp(self.shared_state.cfg.max_request_timestamp_skew)?;

        // does the message signature verify?
        request.verify_signature()?;

        // retrieve the actually stored key and check if the ciphertext matches
        let Some(shared_key) = self.retrieve_shared_key(address).await? else {
            return Err(AuthenticationFailure::NotRegistered)?;
        };
        request.verify_ciphertext(&shared_key.key)?;

        let session_request_start = request.content.request_timestamp();

        // if the client has already authenticated in the past, make sure this authentication timestamp
        // is different and greater than the old one (in case it was replayed)
        if let Some(prior_usage) = shared_key.last_used_authentication {
            request.ensure_timestamp_not_reused(prior_usage)?;
        }

        // check for duplicate clients
        if let Some(client_data) = self
            .shared_state
            .active_clients_store
            .get_remote_client(address)
        {
            warn!("Detected duplicate connection for client: {address}");
            self.handle_duplicate_client(address, client_data, session_request_start)
                .await?;
        }

        let client_id = shared_key.client_id;

        // update the auth timestamp for future uses
        self.shared_state
            .storage
            .update_last_used_authentication_timestamp(client_id, session_request_start)
            .await?;

        // push any old stored messages to the client
        // (this will be removed soon)
        self.push_stored_messages_to_client(address, &shared_key.key)
            .await?;

        // finally check and retrieve client's bandwidth
        let available_bandwidth = self.get_registered_available_bandwidth(client_id).await?;

        let bandwidth_remaining = if available_bandwidth.expired() {
            self.shared_state.storage.reset_bandwidth(client_id).await?;
            0
        } else {
            available_bandwidth.bytes
        };

        Ok(InitialAuthResult::new(
            Some(ClientDetails::new(
                client_id,
                address,
                shared_key.key,
                session_request_start,
                otel_context,
            )),
            ServerResponse::Authenticate {
                protocol_version: Some(negotiated_protocol),
                status: true,
                bandwidth_remaining,
            },
        ))
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
        client_address: DestinationAddressBytes,
        client_shared_keys: &SharedGatewayKey,
    ) -> Result<i64, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        debug!(
            "Processing register client request for: {}",
            client_address.as_base58_string()
        );

        let client_id = self
            .shared_state
            .storage
            .insert_shared_keys(client_address, client_shared_keys)
            .await?;

        // see if we have bandwidth entry for the client already, if not, create one with zero value
        if self
            .shared_state
            .storage
            .get_available_bandwidth(client_id)
            .await?
            .is_none()
        {
            self.shared_state
                .storage
                .create_bandwidth_entry(client_id)
                .await?;
        }

        self.push_stored_messages_to_client(client_address, client_shared_keys)
            .await?;

        Ok(client_id)
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
        R: CryptoRng + RngCore + Send,
    {
        let negotiated_protocol = self.negotiate_client_protocol(client_protocol_version)?;
        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(negotiated_protocol);

        let remote_identity = Self::extract_remote_identity_from_register_init(&init_data)?;
        let remote_address = remote_identity.derive_destination_address();

        debug!(remote_client = %remote_identity);

        if self
            .shared_state
            .active_clients_store
            .is_active(remote_address)
        {
            return Err(InitialAuthenticationError::DuplicateConnection);
        }

        let shared_keys = self.perform_registration_handshake(init_data).await?;
        let client_id = self.register_client(remote_address, &shared_keys).await?;

        debug!(client_id = %client_id, "managed to finalize client registration");

        let client_details = ClientDetails::new(
            client_id,
            remote_address,
            shared_keys,
            OffsetDateTime::now_utc(),
            None
        );

        Ok(InitialAuthResult::new(
            Some(client_details),
            ServerResponse::Register {
                protocol_version: Some(negotiated_protocol),
                status: true,
            },
        ))
    }

    pub(crate) fn handle_supported_protocol_request(&self) -> ServerResponse {
        debug!("returning gateway protocol version");
        ServerResponse::SupportedProtocol {
            version: CURRENT_PROTOCOL_VERSION,
        }
    }

    async fn handle_reply_supported_protocol_request(&mut self)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Err(err) = self
            .send_websocket_message(self.handle_supported_protocol_request())
            .await
        {
            debug!("failed to reply with protocol version: {err}")
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn handle_initial_client_request(
        &mut self,
        request: ClientControlRequest,
    ) -> Result<Option<ClientDetails>, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
        R: CryptoRng + RngCore + Send,
    {
        // extract and set up opentelemetry context if provided
        let context_ext = if let ClientControlRequest::AuthenticateV2(ref auth_req) = request {
            if let Some(otel_context) = &auth_req.otel_context {
                // Extract OpenTelemetry context
                let context_ext = ManualSpanContextExt::new()
                    .with_extracted_context(otel_context.clone());
                info!("Extracted trace id: {:?}", context_ext.trace_id);

                // Build imported context and set it as parent
                let extractor = TraceContextPropagator::new();
                let extracted_context = extractor.extract(&context_ext.context_carrier);
                let trace_id = if let Some(trace_id) = &context_ext.trace_id {
                    *trace_id
                } else {
                    warn!("No trace id provided in the request, falling back to extracted context");
                    extracted_context.span().span_context().trace_id()
                };
                let span_cx = new_span_context_with_id(trace_id);
                let _context_guard = span_cx.clone().attach();
                warn!("==== Context propagation successful ====");

                // Build root_span with extracted context as parent
                let span = info_span!("=== Manual context propagation starting point ===", %trace_id);
                span.set_parent(span_cx.clone());
                let context_ext = context_ext.set_root_span(span);
                context_ext
            } else {
                warn!("No OpenTelemetry context provided in the request");
                ManualSpanContextExt::new()
            }
        } else {
            warn!("No OpenTelemetry context provided in the request");
            ManualSpanContextExt::new()
        };

        let child_span = if context_ext.is_valid() {
            info_span!(parent: &context_ext.root_span, "handling initial client request with otel context")
        } else {
            info_span!("handling_initial_client_request")
        };
        let _enter = child_span.enter();

        let auth_result = match request {
            ClientControlRequest::Authenticate {
                protocol_version,
                address,
                enc_address,
                iv,
                otel_context: _,
            } => {
                self.handle_legacy_authenticate(protocol_version, address, enc_address, iv)
                    .await
            }
            ClientControlRequest::AuthenticateV2(req) => self.handle_authenticate_v2(req, Some(context_ext)).await,
            ClientControlRequest::RegisterHandshakeInitRequest {
                protocol_version,
                data,
            } => self.handle_register(protocol_version, data).await,
            ClientControlRequest::SupportedProtocol { .. } => {
                self.handle_reply_supported_protocol_request().await;
                return Ok(None);
            }
            _ => {
                debug!("received an invalid client request");
                return Err(InitialAuthenticationError::InvalidRequest);
            }
        };

        let auth_result = match auth_result {
            Ok(res) => res,
            Err(err) => {
                match &err {
                    InitialAuthenticationError::StorageError(inner_storage) => {
                        debug!("authentication failure due to storage issue: {inner_storage}")
                    }
                    other => debug!("authentication failure: {other}"),
                }
                self.send_and_forget_error_response(&err).await;
                return Err(err);
            }
        };

                // try to send auth response back to the client
        if let Err(source) = self
            .send_websocket_message(auth_result.server_response)
            .await
        {
            debug!("failed to send authentication response: {source}");
            return Err(InitialAuthenticationError::ResponseSendFailure {
                source: Box::new(source),
            });
        }

        let Some(client_details) = auth_result.client_details else {
            // honestly, it's been so long I don't remember under what conditions its possible (if at all)
            // to have empty client details
            warn!("could not establish client details");
            return Err(InitialAuthenticationError::EmptyClientDetails);
        };

        Ok(Some(client_details))
    }

    #[instrument(skip_all)]
    pub(crate) async fn handle_until_authenticated_or_failure(
        mut self,
    ) -> Option<AuthenticatedHandler<R, S>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
        R: CryptoRng + RngCore + Send,
    {
        loop {
            let req = self.wait_for_initial_message().await;
            let initial_request = match req {
                Ok(req) => req,
                Err(err) => {
                    self.send_and_forget_error_response(err).await;
                    return None;
                }
            };

            // see if we managed to register the client through this request
            let maybe_auth_res = match self.handle_initial_client_request(initial_request).await {
                Ok(maybe_auth_res) => maybe_auth_res,
                Err(err) => {
                    debug!("initial client request handling error: {err}");
                    self.send_and_forget_error_response(err).await;
                    return None;
                }
            };

            if let Some(registration_details) = maybe_auth_res {
                let (mix_sender, mix_receiver) = mpsc::unbounded();
                // Channel for handlers to ask other handlers if they are still active.
                let (is_active_request_sender, is_active_request_receiver) = mpsc::unbounded();
                self.shared_state.active_clients_store.insert_remote(
                    registration_details.address,
                    mix_sender,
                    is_active_request_sender,
                    registration_details.session_request_timestamp,
                );

                let auth_handle = AuthenticatedHandler::upgrade(
                    self,
                    registration_details,
                    mix_receiver,
                    is_active_request_receiver,
                )
                .await
                .inspect_err(|err| error!("failed to upgrade client handler: {err}"))
                .ok();
                return auth_handle;
            }
        }
    }

    pub(crate) async fn wait_for_initial_message(
        &mut self,
    ) -> Result<ClientControlRequest, InitialAuthenticationError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let msg = match timeout(INITIAL_MESSAGE_TIMEOUT, self.read_websocket_message()).await {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(source))) => {
                debug!("failed to obtain message from websocket stream! stopping connection handler: {source}");
                return Err(InitialAuthenticationError::FailedToReadMessage {
                    source: Box::new(source),
                });
            }
            Ok(None) => return Err(InitialAuthenticationError::ClosedConnection),
            Err(_timeout) => return Err(InitialAuthenticationError::Timeout),
        };

        let text = match msg {
            Message::Text(text) => text,
            Message::Binary(_) => {
                return Err(InitialAuthenticationError::BinaryRequestWithoutAuthentication);
            }
            other => {
                if other.is_ping() {
                    debug!("unexpected ping message!");
                    return Err(InitialAuthenticationError::UnexpectedMessageType {
                        typ: "ping".to_string(),
                    });
                } else if other.is_pong() {
                    debug!("unexpected pong message!");
                    return Err(InitialAuthenticationError::UnexpectedMessageType {
                        typ: "pong".to_string(),
                    });
                } else if other.is_close() {
                    debug!("unexpected close message!");
                    return Err(InitialAuthenticationError::UnexpectedMessageType {
                        typ: "close".to_string(),
                    });
                }

                // at this point this is definitely unreachable, but just in case, let's not panic...
                return Err(InitialAuthenticationError::InvalidRequest);
            }
        };

        text.parse()
            .map_err(|_| InitialAuthenticationError::InvalidRequest)
    }

    #[instrument(skip_all)]
    pub(crate) async fn start_handling(self)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
        R: CryptoRng + RngCore + Send,
    {
        let remote = self.peer_address;
        let shutdown = self.shutdown.clone();
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::trace!("received cancellation")
            }
            _ = super::handle_connection(self) => {
                tracing::debug!("finished connection handler for {remote}")
            }
        }
    }
}
