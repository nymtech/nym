// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
pub use crate::packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use crate::socket_state::{ws_fd, PartiallyDelegated, SocketState};
use crate::traits::GatewayPacketRouter;
use crate::{cleanup_socket_message, try_decrypt_binary_message};
use futures::{SinkExt, StreamExt};
use log::*;
use nym_bandwidth_controller::BandwidthController;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_credentials::CredentialSpendingData;
use nym_crypto::asymmetric::identity;
use nym_gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use nym_gateway_requests::iv::IV;
use nym_gateway_requests::registration::handshake::{client_handshake, SharedKeys};
use nym_gateway_requests::{
    BinaryRequest, ClientControlRequest, ServerResponse, CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION,
    CURRENT_PROTOCOL_VERSION,
};
use nym_network_defaults::REMAINING_BANDWIDTH_THRESHOLD;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::TaskClient;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use rand::rngs::OsRng;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::protocol::Message;
use url::Url;

#[cfg(unix)]
use std::os::fd::RawFd;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::connect_async;

#[cfg(not(unix))]
use std::os::raw::c_int as RawFd;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const DEFAULT_RECONNECTION_ATTEMPTS: usize = 10;
const DEFAULT_RECONNECTION_BACKOFF: Duration = Duration::from_secs(5);

pub struct GatewayConfig {
    pub gateway_identity: identity::PublicKey,

    // currently a dead field
    pub gateway_owner: Option<String>,

    pub gateway_listener: String,
}

impl GatewayConfig {
    pub fn new(
        gateway_identity: identity::PublicKey,
        gateway_owner: Option<String>,
        gateway_listener: String,
    ) -> Self {
        GatewayConfig {
            gateway_identity,
            gateway_owner,
            gateway_listener,
        }
    }
}

// TODO: this should be refactored into a state machine that keeps track of its authentication state
#[derive(Debug)]
pub struct GatewayClient<C, St = EphemeralCredentialStorage> {
    authenticated: bool,
    disabled_credentials_mode: bool,
    bandwidth_remaining: Arc<AtomicI64>,
    gateway_address: String,
    gateway_identity: identity::PublicKey,
    local_identity: Arc<identity::KeyPair>,
    shared_key: Option<Arc<SharedKeys>>,
    connection: SocketState,
    packet_router: PacketRouter,
    response_timeout_duration: Duration,
    bandwidth_controller: Option<BandwidthController<C, St>>,

    // reconnection related variables
    /// Specifies whether client should try to reconnect to gateway on connection failure.
    should_reconnect_on_failure: bool,
    /// Specifies maximum number of attempts client will try to reconnect to gateway on failure
    /// before giving up.
    reconnection_attempts: usize,
    /// Delay between each subsequent reconnection attempt.
    reconnection_backoff: Duration,

    // currently unused (but populated)
    negotiated_protocol: Option<u8>,

    /// Listen to shutdown messages.
    shutdown: TaskClient,
}

impl<C, St> GatewayClient<C, St> {
    pub fn new(
        config: GatewayConfig,
        local_identity: Arc<identity::KeyPair>,
        // TODO: make it mandatory. if you don't want to pass it, use `new_init`
        shared_key: Option<Arc<SharedKeys>>,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        shutdown: TaskClient,
    ) -> Self {
        GatewayClient {
            authenticated: false,
            disabled_credentials_mode: true,
            bandwidth_remaining: Arc::new(AtomicI64::new(0)),
            gateway_address: config.gateway_listener,
            gateway_identity: config.gateway_identity,
            local_identity,
            shared_key,
            connection: SocketState::NotConnected,
            packet_router,
            response_timeout_duration: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            bandwidth_controller,
            should_reconnect_on_failure: true,
            reconnection_attempts: DEFAULT_RECONNECTION_ATTEMPTS,
            reconnection_backoff: DEFAULT_RECONNECTION_BACKOFF,
            negotiated_protocol: None,
            shutdown,
        }
    }

    #[must_use]
    pub fn with_disabled_credentials_mode(mut self, disabled_credentials_mode: bool) -> Self {
        self.disabled_credentials_mode = disabled_credentials_mode;
        self
    }

    #[must_use]
    pub fn with_reconnection_on_failure(mut self, should_reconnect_on_failure: bool) -> Self {
        self.should_reconnect_on_failure = should_reconnect_on_failure;
        self
    }

    #[must_use]
    pub fn with_response_timeout(mut self, response_timeout_duration: Duration) -> Self {
        self.response_timeout_duration = response_timeout_duration;
        self
    }

    #[must_use]
    pub fn with_reconnection_attempts(mut self, reconnection_attempts: usize) -> Self {
        self.reconnection_attempts = reconnection_attempts;
        self
    }

    #[must_use]
    pub fn with_reconnection_backoff(mut self, backoff: Duration) -> Self {
        self.reconnection_backoff = backoff;
        self
    }

    pub fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway_identity
    }

    pub fn ws_fd(&self) -> Option<RawFd> {
        match &self.connection {
            SocketState::Available(conn) => ws_fd(conn.as_ref()),
            SocketState::PartiallyDelegated(conn) => conn.ws_fd(),
            _ => None,
        }
    }

    pub fn remaining_bandwidth(&self) -> i64 {
        self.bandwidth_remaining.load(Ordering::Acquire)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn _close_connection(&mut self) -> Result<(), GatewayClientError> {
        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            SocketState::Available(mut socket) => Ok((*socket).close(None).await?),
            SocketState::PartiallyDelegated(_) => {
                unreachable!("this branch should have never been reached!")
            }
            _ => Ok(()), // no need to do anything in those cases
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn _close_connection(&mut self) -> Result<(), GatewayClientError> {
        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            SocketState::Available(socket) => {
                (*socket).close(None, None).await?;
                Ok(())
            }
            SocketState::PartiallyDelegated(_) => {
                unreachable!("this branch should have never been reached!")
            }
            _ => Ok(()), // no need to do anything in those cases
        }
    }

    pub async fn close_connection(&mut self) -> Result<(), GatewayClientError> {
        if self.connection.is_partially_delegated() {
            self.recover_socket_connection().await?;
        }

        self._close_connection().await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
        debug!(
            "Attemting to establish connection to gateway at: {}",
            self.gateway_address
        );
        let ws_stream = match connect_async(&self.gateway_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(error) => {
                return Err(GatewayClientError::NetworkConnectionFailed {
                    address: self.gateway_address.clone(),
                    source: error,
                })
            }
        };

        self.connection = SocketState::Available(Box::new(ws_stream));
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
        let ws_stream = match JSWebsocket::new(&self.gateway_address) {
            Ok(ws_stream) => ws_stream,
            Err(e) => {
                return Err(GatewayClientError::NetworkErrorWasm(e));
            }
        };

        self.connection = SocketState::Available(Box::new(ws_stream));
        Ok(())
    }

    // ignore the current socket state (with which we can't do much anyway)
    // note: the caller MUST ensure that if the stream was delegated, the spawned
    // future is finished.
    async fn attempt_reconnection(&mut self) -> Result<(), GatewayClientError> {
        info!("Attempting gateway reconnection...");
        self.authenticated = false;

        for i in 1..self.reconnection_attempts {
            info!("attempt {}...", i);
            if self.try_reconnect().await.is_ok() {
                info!("managed to reconnect!");
                return Ok(());
            }

            sleep(self.reconnection_backoff).await;
        }

        // final attempt (done separately to be able to return a proper error)
        info!("attempt {}", self.reconnection_attempts);
        match self.try_reconnect().await {
            Ok(_) => {
                info!("managed to reconnect!");
                Ok(())
            }
            Err(err) => {
                error!(
                    "failed to reconnect after {} attempts",
                    self.reconnection_attempts
                );
                Err(err)
            }
        }
    }

    async fn read_control_response(&mut self) -> Result<ServerResponse, GatewayClientError> {
        // we use the fact that all request responses are Message::Text and only pushed
        // sphinx packets are Message::Binary

        let conn = match self.connection {
            SocketState::Available(ref mut conn) => conn,
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        };

        let timeout = sleep(self.response_timeout_duration);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    log::trace!("GatewayClient control response: Received shutdown");
                    log::debug!("GatewayClient control response: Exiting");
                    break Err(GatewayClientError::ConnectionClosedGatewayShutdown);
                }
                _ = &mut timeout => {
                    break Err(GatewayClientError::Timeout);
                }
                msg = conn.next() => {
                    let ws_msg = match cleanup_socket_message(msg) {
                        Err(err) => break Err(err),
                        Ok(msg) => msg
                    };
                    match ws_msg {
                        Message::Binary(bin_msg) => {
                            // if we have established the shared key already, attempt to use it for decryption
                            // otherwise there's not much we can do apart from just routing what we have on hand
                            if let Some(shared_keys) = &self.shared_key {
                                if let Some(plaintext) = try_decrypt_binary_message(bin_msg, shared_keys) {
                                    if let Err(err) = self.packet_router.route_received(vec![plaintext]) {
                                        log::warn!("Route received failed: {err}");
                                    }
                                }
                            } else if let Err(err) = self.packet_router.route_received(vec![bin_msg]) {
                                log::warn!("Route received failed: {err}");
                            }
                        }
                        Message::Text(txt_msg) => {
                            break ServerResponse::try_from(txt_msg).map_err(|_| GatewayClientError::MalformedResponse);
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    // If we want to send a message (with response), we need to have a full control over the socket,
    // as we need to be able to write the request and read the subsequent response
    async fn send_websocket_message(
        &mut self,
        msg: Message,
    ) -> Result<ServerResponse, GatewayClientError> {
        let should_restart_mixnet_listener = if self.connection.is_partially_delegated() {
            self.recover_socket_connection().await?;
            true
        } else {
            false
        };

        let conn = match self.connection {
            SocketState::Available(ref mut conn) => conn,
            SocketState::NotConnected => return Err(GatewayClientError::ConnectionNotEstablished),
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        };
        conn.send(msg).await?;
        let response = self.read_control_response().await;

        if should_restart_mixnet_listener {
            self.start_listening_for_mixnet_messages()?;
        }
        response
    }

    async fn batch_send_websocket_messages_without_response(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<(), GatewayClientError> {
        match self.connection {
            SocketState::Available(ref mut conn) => {
                let stream_messages: Vec<_> = messages.into_iter().map(Ok).collect();
                let mut send_stream = futures::stream::iter(stream_messages);
                Ok(conn.send_all(&mut send_stream).await?)
            }
            SocketState::PartiallyDelegated(ref mut partially_delegated) => {
                if let Err(err) = partially_delegated
                    .batch_send_without_response(messages)
                    .await
                {
                    error!("failed to batch send messages - {err}...");
                    // we must ensure we do not leave the task still active
                    if let Err(err) = self.recover_socket_connection().await {
                        error!("... and the delegated stream has also errored out - {err}")
                    }
                    Err(err)
                } else {
                    Ok(())
                }
            }
            SocketState::NotConnected => Err(GatewayClientError::ConnectionNotEstablished),
            _ => Err(GatewayClientError::ConnectionInInvalidState),
        }
    }

    async fn send_websocket_message_without_response(
        &mut self,
        msg: Message,
    ) -> Result<(), GatewayClientError> {
        match self.connection {
            SocketState::Available(ref mut conn) => Ok(conn.send(msg).await?),
            SocketState::PartiallyDelegated(ref mut partially_delegated) => {
                if let Err(err) = partially_delegated.send_without_response(msg).await {
                    error!("failed to send message without response - {err}...");
                    // we must ensure we do not leave the task still active
                    if let Err(err) = self.recover_socket_connection().await {
                        error!("... and the delegated stream has also errored out - {err}")
                    }
                    Err(err)
                } else {
                    Ok(())
                }
            }
            SocketState::NotConnected => Err(GatewayClientError::ConnectionNotEstablished),
            _ => Err(GatewayClientError::ConnectionInInvalidState),
        }
    }

    fn check_gateway_protocol(
        &self,
        gateway_protocol: Option<u8>,
    ) -> Result<(), GatewayClientError> {
        debug!("gateway protocol: {gateway_protocol:?}, ours: {CURRENT_PROTOCOL_VERSION}");

        // right now there are no failure cases here, but this might change in the future
        match gateway_protocol {
            None => {
                warn!("the gateway we're connected to has not specified its protocol version. It's probably running version < 1.1.X, but that's still fine for now. It will become a hard error in 1.2.0");
                // note: in +1.2.0 we will have to return a hard error here
                Ok(())
            }
            Some(v) if v > CURRENT_PROTOCOL_VERSION => {
                let err = GatewayClientError::IncompatibleProtocol {
                    gateway: Some(v),
                    current: CURRENT_PROTOCOL_VERSION,
                };
                error!("{err}");
                Err(err)
            }

            Some(_) => {
                info!("the gateway is using exactly the same (or older) protocol version as we are. We're good to continue!");
                Ok(())
            }
        }
    }

    async fn register(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        debug_assert!(self.connection.is_available());
        log::debug!("Registering gateway");

        // it's fine to instantiate it here as it's only used once (during authentication or registration)
        // and putting it into the GatewayClient struct would be a hassle
        let mut rng = OsRng;

        let shared_key = match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut rng,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
                !self.disabled_credentials_mode,
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => unreachable!(),
        }?;
        let (authentication_status, gateway_protocol) = match self.read_control_response().await? {
            ServerResponse::Register {
                protocol_version,
                status,
            } => (status, protocol_version),
            ServerResponse::Error { message } => {
                return Err(GatewayClientError::GatewayError(message))
            }
            _ => return Err(GatewayClientError::UnexpectedResponse),
        };

        self.check_gateway_protocol(gateway_protocol)?;
        self.authenticated = authentication_status;

        if self.authenticated {
            self.shared_key = Some(Arc::new(shared_key));
        }

        // populate the negotiated protocol for future uses
        self.negotiated_protocol = gateway_protocol;

        Ok(())
    }

    async fn authenticate(
        &mut self,
        shared_key: Option<SharedKeys>,
    ) -> Result<(), GatewayClientError> {
        if shared_key.is_none() && self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        log::debug!("Authenticating with gateway");

        // it's fine to instantiate it here as it's only used once (during authentication or registration)
        // and putting it into the GatewayClient struct would be a hassle
        let mut rng = OsRng;

        // because of the previous check one of the unwraps MUST succeed
        let shared_key = shared_key
            .as_ref()
            .unwrap_or_else(|| self.shared_key.as_ref().unwrap());
        let iv = IV::new_random(&mut rng);
        let self_address = self
            .local_identity
            .as_ref()
            .public_key()
            .derive_destination_address();
        let encrypted_address = EncryptedAddressBytes::new(&self_address, shared_key, &iv);

        let msg = ClientControlRequest::new_authenticate(
            self_address,
            encrypted_address,
            iv,
            !self.disabled_credentials_mode,
        )
        .into();

        match self.send_websocket_message(msg).await? {
            ServerResponse::Authenticate {
                protocol_version,
                status,
                bandwidth_remaining,
            } => {
                self.check_gateway_protocol(protocol_version)?;
                self.authenticated = status;
                self.bandwidth_remaining
                    .store(bandwidth_remaining, Ordering::Release);
                self.negotiated_protocol = protocol_version;
                log::debug!("authenticated: {status}, bandwidth remaining: {bandwidth_remaining}");
                Ok(())
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }
    }

    /// Helper method to either call register or authenticate based on self.shared_key value
    pub async fn perform_initial_authentication(
        &mut self,
    ) -> Result<Arc<SharedKeys>, GatewayClientError> {
        if self.authenticated {
            debug!("Already authenticated");
            return if let Some(shared_key) = &self.shared_key {
                Ok(Arc::clone(shared_key))
            } else {
                Err(GatewayClientError::AuthenticationFailureWithPreexistingSharedKey)
            };
        }

        if self.shared_key.is_some() {
            self.authenticate(None).await?;
        } else {
            self.register().await?;
        }
        if self.authenticated {
            // if we are authenticated it means we MUST have an associated shared_key
            Ok(Arc::clone(self.shared_key.as_ref().unwrap()))
        } else {
            Err(GatewayClientError::AuthenticationFailure)
        }
    }

    async fn claim_ecash_bandwidth(
        &mut self,
        credential: CredentialSpendingData,
    ) -> Result<(), GatewayClientError> {
        let mut rng = OsRng;
        let iv = IV::new_random(&mut rng);

        let msg = ClientControlRequest::new_enc_ecash_credential(
            credential,
            self.shared_key.as_ref().unwrap(),
            iv,
        )
        .into();
        let bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;
        self.bandwidth_remaining
            .store(bandwidth_remaining, Ordering::Relaxed);
        Ok(())
    }

    async fn try_claim_testnet_bandwidth(&mut self) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::ClaimFreeTestnetBandwidth.into();
        let bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;
        self.bandwidth_remaining
            .store(bandwidth_remaining, Ordering::Release);
        Ok(())
    }

    pub async fn claim_bandwidth(&mut self) -> Result<(), GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if self.bandwidth_controller.is_none() && !self.disabled_credentials_mode {
            return Err(GatewayClientError::NoBandwidthControllerAvailable);
        }

        warn!("Not enough bandwidth. Trying to get more bandwidth, this might take a while");
        if self.disabled_credentials_mode {
            info!("The client is running in disabled credentials mode - attempting to claim bandwidth without a credential");
            return self.try_claim_testnet_bandwidth().await;
        }

        let Some(gateway_protocol) = self.negotiated_protocol else {
            return Err(GatewayClientError::OutdatedGatewayCredentialVersion {
                negotiated_protocol: None,
            });
        };

        if gateway_protocol < CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION {
            return Err(GatewayClientError::OutdatedGatewayCredentialVersion {
                negotiated_protocol: Some(gateway_protocol),
            });
        }
        let prepared_credential = self
            .bandwidth_controller
            .as_ref()
            .unwrap()
            .prepare_ecash_credential(self.gateway_identity.to_bytes())
            .await?;

        self.claim_ecash_bandwidth(prepared_credential.data).await?;
        self.bandwidth_controller
            .as_ref()
            .unwrap()
            .update_ecash_wallet(
                prepared_credential.updated_credential,
                prepared_credential.credential_id,
            )
            .await?;

        Ok(())
    }

    pub async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        debug!("Sending {} mix packets", packets.len());

        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        let bandwidth_remaining = self.bandwidth_remaining.load(Ordering::Acquire);
        if bandwidth_remaining < REMAINING_BANDWIDTH_THRESHOLD {
            self.claim_bandwidth().await?;
        }

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        let messages: Vec<_> = packets
            .into_iter()
            .map(|mix_packet| {
                BinaryRequest::new_forward_request(mix_packet).into_ws_message(
                    self.shared_key
                        .as_ref()
                        .expect("no shared key present even though we're authenticated!"),
                )
            })
            .collect();

        if let Err(err) = self
            .batch_send_websocket_messages_without_response(messages)
            .await
        {
            if err.is_closed_connection() && self.should_reconnect_on_failure {
                self.attempt_reconnection().await
            } else {
                Err(err)
            }
        } else {
            Ok(())
        }
    }

    async fn send_with_reconnection_on_failure(
        &mut self,
        msg: Message,
    ) -> Result<(), GatewayClientError> {
        if let Err(err) = self.send_websocket_message_without_response(msg).await {
            if err.is_closed_connection() && self.should_reconnect_on_failure {
                debug!("Going to attempt a reconnection");
                self.attempt_reconnection().await
            } else {
                warn!("{err}");
                Err(err)
            }
        } else {
            Ok(())
        }
    }

    pub async fn send_ping_message(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        // as per RFC6455 section 5.5.2, `Ping frame MAY include "Application data".`
        // so we don't need to include any here.
        let msg = Message::Ping(Vec::new());
        self.send_with_reconnection_on_failure(msg).await
    }

    // TODO: possibly make responses optional
    pub async fn send_mix_packet(&mut self, mix_packet: MixPacket) -> Result<(), GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        let bandwidth_remaining = self.bandwidth_remaining.load(Ordering::Acquire);
        if bandwidth_remaining < REMAINING_BANDWIDTH_THRESHOLD {
            self.claim_bandwidth().await?;
        }

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let msg = BinaryRequest::new_forward_request(mix_packet).into_ws_message(
            self.shared_key
                .as_ref()
                .expect("no shared key present even though we're authenticated!"),
        );
        self.send_with_reconnection_on_failure(msg).await
    }

    async fn recover_socket_connection(&mut self) -> Result<(), GatewayClientError> {
        if self.connection.is_available() {
            return Ok(());
        }
        if !self.connection.is_partially_delegated() {
            return Err(GatewayClientError::ConnectionInInvalidState);
        }

        let conn = match std::mem::replace(&mut self.connection, SocketState::Invalid) {
            SocketState::PartiallyDelegated(delegated_conn) => delegated_conn.merge().await?,
            _ => unreachable!(),
        };

        self.connection = SocketState::Available(Box::new(conn));
        Ok(())
    }

    // Note: this requires prior authentication
    pub fn start_listening_for_mixnet_messages(&mut self) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.connection.is_partially_delegated() {
            return Ok(());
        }
        if !self.connection.is_available() {
            return Err(GatewayClientError::ConnectionInInvalidState);
        }

        let partially_delegated =
            match std::mem::replace(&mut self.connection, SocketState::Invalid) {
                SocketState::Available(conn) => {
                    PartiallyDelegated::split_and_listen_for_mixnet_messages(
                        *conn,
                        self.packet_router.clone(),
                        Arc::clone(
                            self.shared_key
                                .as_ref()
                                .expect("no shared key present even though we're authenticated!"),
                        ),
                        self.bandwidth_remaining.clone(),
                        self.shutdown.clone(),
                    )
                }
                _ => unreachable!(),
            };

        self.connection = SocketState::PartiallyDelegated(partially_delegated);
        Ok(())
    }

    pub async fn try_reconnect(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }

        // TODO: the name of this method is very deceiving
        self.perform_initial_authentication().await?;

        // this call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), GatewayClientError> {
        self.recover_socket_connection().await?;
        self.connection = SocketState::NotConnected;
        Ok(())
    }

    pub async fn authenticate_and_start(&mut self) -> Result<Arc<SharedKeys>, GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }
        let shared_key = self.perform_initial_authentication().await?;
        let bandwidth_remaining = self.bandwidth_remaining.load(Ordering::Acquire);
        if bandwidth_remaining < REMAINING_BANDWIDTH_THRESHOLD {
            info!("Claiming more bandwidth with existing credentials. Stop the process now if you don't want that to happen.");
            self.claim_bandwidth().await?;
        }

        // this call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(shared_key)
    }
}

// type alias for an ease of use
pub type InitGatewayClient = GatewayClient<InitOnly>;

#[derive(Debug)]
pub struct InitOnly;

impl GatewayClient<InitOnly, EphemeralCredentialStorage> {
    // for initialisation we do not need credential storage. Though it's still a bit weird we have to set the generic...
    pub fn new_init(
        gateway_listener: Url,
        gateway_identity: identity::PublicKey,
        local_identity: Arc<identity::KeyPair>,
    ) -> Self {
        log::trace!("Initialising gateway client");
        use futures::channel::mpsc;

        // note: this packet_router is completely invalid in normal circumstances, but "works"
        // perfectly fine here, because it's not meant to be used
        let (ack_tx, _) = mpsc::unbounded();
        let (mix_tx, _) = mpsc::unbounded();
        let shutdown = TaskClient::dummy();
        let packet_router = PacketRouter::new(ack_tx, mix_tx, shutdown.clone());

        GatewayClient {
            authenticated: false,
            disabled_credentials_mode: true,
            bandwidth_remaining: Arc::new(AtomicI64::new(0)),
            gateway_address: gateway_listener.to_string(),
            gateway_identity,
            local_identity,
            shared_key: None,
            connection: SocketState::NotConnected,
            packet_router,
            response_timeout_duration: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            bandwidth_controller: None,
            should_reconnect_on_failure: false,
            reconnection_attempts: DEFAULT_RECONNECTION_ATTEMPTS,
            reconnection_backoff: DEFAULT_RECONNECTION_BACKOFF,
            negotiated_protocol: None,
            shutdown,
        }
    }

    pub fn upgrade<C, St>(
        self,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        shutdown: TaskClient,
    ) -> GatewayClient<C, St> {
        // invariants that can't be broken
        // (unless somebody decided to expose some field that wasn't meant to be exposed)
        assert!(self.authenticated);
        assert!(self.connection.is_available());
        assert!(self.shared_key.is_some());

        GatewayClient {
            authenticated: self.authenticated,
            disabled_credentials_mode: self.disabled_credentials_mode,
            bandwidth_remaining: self.bandwidth_remaining,
            gateway_address: self.gateway_address,
            gateway_identity: self.gateway_identity,
            local_identity: self.local_identity,
            shared_key: self.shared_key,
            connection: self.connection,
            packet_router,
            response_timeout_duration: self.response_timeout_duration,
            bandwidth_controller,
            should_reconnect_on_failure: self.should_reconnect_on_failure,
            reconnection_attempts: self.reconnection_attempts,
            reconnection_backoff: self.reconnection_backoff,
            negotiated_protocol: self.negotiated_protocol,
            shutdown,
        }
    }
}
