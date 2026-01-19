// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bandwidth::ClientBandwidth;
use crate::client::config::GatewayClientConfig;
use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
pub use crate::packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use crate::socket_state::{ws_fd, PartiallyDelegatedHandle, SocketState};
use crate::traits::GatewayPacketRouter;
use crate::{cleanup_socket_message, try_decrypt_binary_message};
use futures::{SinkExt, StreamExt};
use nym_bandwidth_controller::BandwidthController;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_requests::registration::handshake::client_handshake;
use nym_gateway_requests::{
    BandwidthResponse, BinaryRequest, ClientControlRequest, ClientRequest, GatewayProtocolVersion,
    GatewayProtocolVersionExt, GatewayRequestsError, ServerResponse, SharedSymmetricKey,
    CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION, CURRENT_PROTOCOL_VERSION,
};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_statistics_common::clients::connection::ConnectionStatsEvent;
use nym_statistics_common::clients::ClientStatsSender;
use nym_task::ShutdownToken;
use nym_topology::EntryDetails;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use rand::rngs::OsRng;
use std::sync::Arc;
use tracing::instrument;
use tracing::*;
use tungstenite::protocol::Message;

#[cfg(unix)]
use std::os::fd::RawFd;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;

#[cfg(not(unix))]
use std::os::raw::c_int as RawFd;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;
#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

pub mod config;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod websockets;
#[cfg(not(target_arch = "wasm32"))]
use crate::client::websockets::connect_async;

pub struct GatewayConfig {
    pub gateway_identity: ed25519::PublicKey,

    pub gateway_details: EntryDetails,
}

impl GatewayConfig {
    pub fn new(gateway_identity: ed25519::PublicKey, gateway_details: EntryDetails) -> Self {
        GatewayConfig {
            gateway_identity,
            gateway_details,
        }
    }
}

#[must_use]
#[derive(Debug)]
pub struct AuthenticationResponse {
    pub initial_shared_key: Arc<SharedSymmetricKey>,
}

// TODO: this should be refactored into a state machine that keeps track of its authentication state
pub struct GatewayClient<C, St = EphemeralCredentialStorage> {
    pub cfg: GatewayClientConfig,

    authenticated: bool,
    bandwidth: ClientBandwidth,
    gateway_details: EntryDetails,
    gateway_identity: ed25519::PublicKey,
    local_identity: Arc<ed25519::KeyPair>,
    shared_key: Option<Arc<SharedSymmetricKey>>,
    connection: SocketState,
    packet_router: PacketRouter,
    bandwidth_controller: Option<BandwidthController<C, St>>,
    stats_reporter: ClientStatsSender,

    negotiated_protocol: Option<GatewayProtocolVersion>,

    // Callback on the fd as soon as the connection has been established
    #[cfg(unix)]
    connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,

    /// Listen to shutdown messages and send notifications back to the task manager
    shutdown_token: ShutdownToken,
}

impl<C, St> GatewayClient<C, St> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cfg: GatewayClientConfig,
        gateway_config: GatewayConfig,
        local_identity: Arc<ed25519::KeyPair>,
        // TODO: make it mandatory. if you don't want to pass it, use `new_init`
        shared_key: Option<Arc<SharedSymmetricKey>>,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        stats_reporter: ClientStatsSender,
        #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        GatewayClient {
            cfg,
            authenticated: false,
            bandwidth: ClientBandwidth::new_empty(),
            gateway_details: gateway_config.gateway_details,
            gateway_identity: gateway_config.gateway_identity,
            local_identity,
            shared_key,
            connection: SocketState::NotConnected,
            packet_router,
            bandwidth_controller,
            stats_reporter,
            negotiated_protocol: None,
            #[cfg(unix)]
            connection_fd_callback,
            shutdown_token,
        }
    }

    pub fn gateway_identity(&self) -> ed25519::PublicKey {
        self.gateway_identity
    }

    pub fn shared_key(&self) -> Option<Arc<SharedSymmetricKey>> {
        self.shared_key.clone()
    }

    pub fn ws_fd(&self) -> Option<RawFd> {
        match &self.connection {
            SocketState::Available(conn) => ws_fd(conn.as_ref()),
            SocketState::PartiallyDelegated(conn) => conn.ws_fd(),
            _ => None,
        }
    }

    pub fn remaining_bandwidth(&self) -> i64 {
        self.bandwidth.remaining()
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::unreachable)]
    async fn _close_connection(&mut self) -> Result<(), GatewayClientError> {
        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            SocketState::Available(mut socket) => Ok((*socket).close(None).await?),
            SocketState::PartiallyDelegated(_) => {
                // SAFETY: this is only called after the caller has already recovered the connection
                unreachable!("this branch should have never been reached!")
            }
            _ => Ok(()), // no need to do anything in those cases
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unreachable)]
    async fn _close_connection(&mut self) -> Result<(), GatewayClientError> {
        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            SocketState::Available(socket) => {
                (*socket).close(None, None).await?;
                Ok(())
            }
            SocketState::PartiallyDelegated(_) => {
                // SAFETY: this is only called after the caller has already recovered the connection
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
            "Attempting to establish connection to gateway at: {:?}",
            self.gateway_details
        );

        let (ws_stream, _) = connect_async(
            &self.gateway_details,
            #[cfg(unix)]
            self.connection_fd_callback.clone(),
        )
        .await?;

        self.connection = SocketState::Available(Box::new(ws_stream));

        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
        let endpoint = self.gateway_details.clone();
        let uri = endpoint
            .ws_entry_address(false)
            .ok_or(GatewayClientError::InvalidEndpoint(endpoint.to_string()))?;
        let ws_stream = match JSWebsocket::new(uri.as_ref()) {
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

        for i in 1..self.cfg.connection.reconnection_attempts {
            info!("reconnection attempt {}...", i);
            if self.try_reconnect().await.is_ok() {
                info!("managed to reconnect!");
                return Ok(());
            }

            sleep(self.cfg.connection.reconnection_backoff).await;
        }

        // final attempt (done separately to be able to return a proper error)
        info!(
            "reconnection attempt {}",
            self.cfg.connection.reconnection_attempts
        );
        match self.try_reconnect().await {
            Ok(_) => {
                info!("managed to reconnect!");
                Ok(())
            }
            Err(err) => {
                error!(
                    "failed to reconnect after {} attempts",
                    self.cfg.connection.reconnection_attempts
                );
                Err(err)
            }
        }
    }

    pub async fn send_client_request(
        &mut self,
        message: ClientRequest,
    ) -> Result<(), GatewayClientError> {
        if let Some(shared_key) = self.shared_key() {
            let encrypted = message.encrypt(&shared_key)?;
            Box::pin(self.send_websocket_message_without_response(encrypted)).await?;
            Ok(())
        } else {
            Err(GatewayClientError::ConnectionInInvalidState)
        }
    }

    async fn read_control_response(&mut self) -> Result<ServerResponse, GatewayClientError> {
        // we use the fact that all request responses are Message::Text and only pushed
        // sphinx packets are Message::Binary

        let conn = match self.connection {
            SocketState::Available(ref mut conn) => conn,
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        };

        let timeout = sleep(self.cfg.connection.response_timeout_duration);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
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

    /// Attempt to send a websocket message to the gateway without waiting for any response
    async fn send_websocket_message_without_response(
        &mut self,
        msg: impl Into<Message>,
    ) -> Result<(), GatewayClientError> {
        match self.connection {
            SocketState::Available(ref mut conn) => Ok(conn.send(msg.into()).await?),
            SocketState::PartiallyDelegated(ref mut partially_delegated) => {
                if let Err(err) = partially_delegated.send_without_response(msg.into()).await {
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

    // A very nasty hack due to lack of id tags on messages - send a non-sphinx packet websocket
    // message and wait until first non 'Send' response within timeout
    pub async fn send_websocket_message_with_non_send_response(
        &mut self,
        msg: impl Into<Message>,
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
        conn.send(msg.into()).await?;

        let timeout = sleep(self.cfg.connection.response_timeout_duration);
        tokio::pin!(timeout);

        let response = loop {
            tokio::select! {
                _ = &mut timeout => {
                    break Err(GatewayClientError::Timeout);
                }
                // note: the below will also listen for shutdown signals
                msg = self.read_control_response() => {
                    match msg {
                        Ok(res) => if !res.is_send() {
                            break Ok(res);
                        },
                        Err(err) => break Err(err),
                    }
                }
            }
        };

        if should_restart_mixnet_listener {
            self.start_listening_for_mixnet_messages()?;
        }
        response
    }

    /// Attempt to send a websocket message to the gateway and wait until we receive a response.
    // If we want to send a message (with response), we need to have a full control over the socket,
    // as we need to be able to write the request and read the subsequent response
    pub async fn send_websocket_message_with_response(
        &mut self,
        msg: impl Into<Message>,
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
        conn.send(msg.into()).await?;
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

    async fn register(
        &mut self,
        supported_gateway_protocol: GatewayProtocolVersion,
    ) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        debug_assert!(self.connection.is_available());
        log::debug!("registering with gateway");

        // it's fine to instantiate it here as it's only used once (during authentication or registration)
        // and putting it into the GatewayClient struct would be a hassle
        let mut rng = OsRng;

        let handshake_result = match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut rng,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
                supported_gateway_protocol,
                #[cfg(not(target_arch = "wasm32"))]
                self.shutdown_token.clone(),
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        }?;

        let authentication_status = match self.read_control_response().await? {
            ServerResponse::Register {
                status,
                upgrade_mode,
                ..
            } => {
                if upgrade_mode {
                    warn!("the system is currently undergoing an upgrade. some of its functionalities might be unstable")
                }
                status
            }
            ServerResponse::Error { message } => {
                return Err(GatewayClientError::GatewayError(message))
            }
            other => return Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        };

        self.authenticated = authentication_status;

        if self.authenticated {
            self.shared_key = Some(Arc::new(handshake_result.derived_key));
        }

        // populate the negotiated protocol for future uses
        self.negotiated_protocol = Some(handshake_result.negotiated_protocol);

        Ok(())
    }

    async fn send_authenticate_request_and_handle_response(
        &mut self,
        msg: ClientControlRequest,
    ) -> Result<(), GatewayClientError> {
        match self.send_websocket_message_with_response(msg).await? {
            ServerResponse::Authenticate {
                protocol_version,
                status,
                bandwidth_remaining,
                upgrade_mode,
            } => {
                if protocol_version.is_future_version() {
                    error!("the gateway insists on using v{protocol_version} protocol which is not supported by this client");
                    return Err(GatewayClientError::AuthenticationFailure);
                }
                self.authenticated = status;
                self.bandwidth
                    .update_and_maybe_log(bandwidth_remaining, upgrade_mode);

                self.negotiated_protocol = Some(protocol_version);
                log::debug!("authenticated: {status}, bandwidth remaining: {bandwidth_remaining}");
                if upgrade_mode {
                    warn!("the system is currently undergoing an upgrade. some of its functionalities might be unstable")
                }

                Ok(())
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }
    }

    async fn authenticate_v2(
        &mut self,
        requested_protocol_version: GatewayProtocolVersion,
    ) -> Result<(), GatewayClientError> {
        debug!("using v2 authentication");
        let Some(shared_key) = self.shared_key.as_ref() else {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        };

        let msg = ClientControlRequest::new_authenticate_v2(
            shared_key,
            &self.local_identity,
            requested_protocol_version,
        )?;
        self.send_authenticate_request_and_handle_response(msg)
            .await
    }

    async fn authenticate(
        &mut self,
        requested_protocol_version: GatewayProtocolVersion,
    ) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        debug!("authenticating with gateway");

        // use the highest possible protocol version the gateway has announced support for
        self.authenticate_v2(requested_protocol_version).await
    }

    /// Helper method to either call register or authenticate based on self.shared_key value
    #[instrument(skip_all,
        fields(
            gateway = %self.gateway_identity,
            gateway_address = %self.gateway_details
        )
    )]
    pub async fn perform_initial_authentication(
        &mut self,
    ) -> Result<AuthenticationResponse, GatewayClientError> {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }

        // 1. check gateway's protocol version
        // if we failed to get this request resolved, it means the gateway is on an old version
        // that definitely does not support auth v2 or aes256gcm, so we bail
        let gw_protocol = self.get_gateway_protocol().await?;

        debug!("supported gateway protocol: {gw_protocol:?}");

        let supports_aes_gcm_siv = gw_protocol.supports_aes256_gcm_siv();
        let supports_auth_v2 = gw_protocol.supports_authenticate_v2();
        let supports_key_rotation_info = gw_protocol.supports_key_rotation_packet();
        let supports_upgrade_mode = gw_protocol.supports_upgrade_mode();

        if !supports_aes_gcm_siv {
            warn!("this gateway is on an old version that doesn't support AES256-GCM-SIV");
        }
        if !supports_auth_v2 {
            warn!("this gateway is on an old version that doesn't support authentication v2")
        }

        // Dropping v1 support
        if !supports_auth_v2 || !supports_aes_gcm_siv {
            // we can't continue
            return Err(GatewayClientError::IncompatibleProtocol {
                gateway: gw_protocol,
                current: CURRENT_PROTOCOL_VERSION,
            });
        }

        if !supports_key_rotation_info {
            warn!("this gateway is on an old version that doesn't support key rotation packets")
        }
        if !supports_upgrade_mode {
            warn!("this gateway is on an old version that doesn't support upgrade mode")
        }

        let gw_protocol = if gw_protocol.is_future_version() {
            warn!("we're running outdated software as gateway is announcing protocol {gw_protocol:?} whilst we're using {}. we're going to attempt to downgrade", GatewayProtocolVersion::CURRENT);
            GatewayProtocolVersion::CURRENT
        } else {
            gw_protocol
        };

        if self.authenticated {
            debug!("Already authenticated");
            return if let Some(shared_key) = &self.shared_key {
                Ok(AuthenticationResponse {
                    initial_shared_key: Arc::clone(shared_key),
                })
            } else {
                Err(GatewayClientError::AuthenticationFailureWithPreexistingSharedKey)
            };
        }

        if self.shared_key.is_some() {
            self.authenticate(gw_protocol).await?;

            if self.authenticated {
                // if we are authenticated it means we MUST have an associated shared_key
                #[allow(clippy::unwrap_used)]
                let shared_key = self.shared_key.as_ref().unwrap();

                Ok(AuthenticationResponse {
                    initial_shared_key: Arc::clone(shared_key),
                })
            } else {
                Err(GatewayClientError::AuthenticationFailure)
            }
        } else {
            self.register(gw_protocol).await?;

            // if registration didn't return an error, we MUST have an associated shared key
            #[allow(clippy::unwrap_used)]
            let shared_key = self.shared_key.as_ref().unwrap();

            // we're always registering with the highest supported protocol,
            // so no upgrades are required
            Ok(AuthenticationResponse {
                initial_shared_key: Arc::clone(shared_key),
            })
        }
    }

    /// Attempt to retrieve the currently supported gateway protocol version of the remote.
    pub async fn get_gateway_protocol(&mut self) -> Result<u8, GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        match self
            .send_websocket_message_with_non_send_response(
                ClientControlRequest::SupportedProtocol {},
            )
            .await?
        {
            ServerResponse::SupportedProtocol { version } => Ok(version),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }
    }

    async fn wait_for_bandwidth_response(
        &mut self,
        msg: ClientControlRequest,
    ) -> Result<BandwidthResponse, GatewayClientError> {
        let response = match self
            .send_websocket_message_with_non_send_response(msg)
            .await?
        {
            ServerResponse::Bandwidth(response) => {
                if response.upgrade_mode {
                    info!("the system is currently undergoing an upgrade. our bandwidth shouldn't have been metered")
                }
                Ok(response)
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            ServerResponse::TypedError { error } => {
                Err(GatewayClientError::TypedGatewayError(error))
            }
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }?;
        Ok(response)
    }

    async fn claim_ecash_bandwidth(
        &mut self,
        credential: CredentialSpendingData,
    ) -> Result<(), GatewayClientError> {
        // SAFETY: claiming ecash bandwidth is called as part of `claim_bandwidth` which
        // ensures the shared key is defined
        #[allow(clippy::unwrap_used)]
        let msg = ClientControlRequest::new_enc_ecash_credential(
            credential,
            self.shared_key.as_ref().unwrap(),
        )?;
        let response = self.wait_for_bandwidth_response(msg).await?;

        // TODO: create tracing span
        info!("managed to claim ecash bandwidth");
        self.bandwidth
            .update_and_log(response.available_total, response.upgrade_mode);

        Ok(())
    }

    pub async fn send_upgrade_mode_jwt(&mut self, token: String) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::new_upgrade_mode_jwt(token);
        let response = self.wait_for_bandwidth_response(msg).await?;

        // if gateway rejected our jwt, we would have returned an error
        info!("gateway has accepted our jwt");
        if !response.upgrade_mode {
            error!("but we're not in upgrade mode - something is wrong!");
            return Err(GatewayClientError::UnexpectedUpgradeModeState);
        }

        self.bandwidth
            .update_and_log(response.available_total, response.upgrade_mode);

        Ok(())
    }

    async fn try_claim_testnet_bandwidth(&mut self) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::ClaimFreeTestnetBandwidth;
        let response = self.wait_for_bandwidth_response(msg).await?;

        info!("managed to claim testnet bandwidth");
        self.bandwidth
            .update_and_log(response.available_total, response.upgrade_mode);

        Ok(())
    }

    fn unchecked_bandwidth_controller(&self) -> &BandwidthController<C, St> {
        // this is an unchecked method
        #[allow(clippy::unwrap_used)]
        self.bandwidth_controller.as_ref().unwrap()
    }

    pub async fn claim_bandwidth(&mut self) -> Result<(), GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        // TODO: make it configurable
        const TICKETS_TO_SPEND: u32 = 1;
        const MIXNET_TICKET: TicketType = TicketType::V1MixnetEntry;

        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if self.bandwidth_controller.is_none() && self.cfg.bandwidth.require_tickets {
            return Err(GatewayClientError::NoBandwidthControllerAvailable);
        }

        let Some(_claim_guard) = self.bandwidth.begin_bandwidth_claim() else {
            debug!("there's already an existing bandwidth claim ongoing");
            return Ok(());
        };

        warn!("Not enough bandwidth. Trying to get more bandwidth, this might take a while");
        if !self.cfg.bandwidth.require_tickets {
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
            .unchecked_bandwidth_controller()
            .prepare_ecash_ticket(
                MIXNET_TICKET,
                self.gateway_identity.to_bytes(),
                TICKETS_TO_SPEND,
            )
            .await?;

        match self.claim_ecash_bandwidth(prepared_credential.data).await {
            Ok(_) => {
                self.stats_reporter.report(
                    ConnectionStatsEvent::TicketSpent {
                        typ: MIXNET_TICKET,
                        amount: TICKETS_TO_SPEND,
                    }
                    .into(),
                );
                Ok(())
            }
            Err(err) => {
                error!("failed to claim ecash bandwidth with the gateway...: {err}");
                if err.is_ticket_replay() {
                    warn!("this was due to our ticket being replayed! have you messed with the database file?")
                } else {
                    // TODO: tracing span
                    info!("attempting to revert ticket withdrawal...");
                    self.unchecked_bandwidth_controller()
                        .attempt_revert_ticket_usage(prepared_credential.metadata)
                        .await?;
                }

                Err(err)
            }
        }
    }

    fn mix_packet_to_ws_message(&self, packet: MixPacket) -> Result<Message, GatewayRequestsError> {
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let req = if self.negotiated_protocol.supports_key_rotation_packet() {
            BinaryRequest::ForwardSphinxV2 { packet }
        } else {
            BinaryRequest::ForwardSphinx { packet }
        };

        #[allow(clippy::expect_used)]
        req.into_ws_message(
            self.shared_key
                .as_ref()
                .expect("no shared key present even though we're authenticated!"),
        )
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
        let bandwidth_remaining = self.bandwidth.remaining();
        if bandwidth_remaining < self.cfg.bandwidth.remaining_bandwidth_threshold {
            self.cfg
                .bandwidth
                .ensure_above_cutoff(bandwidth_remaining)?;
            self.claim_bandwidth().await?;
        }

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        let messages: Result<Vec<_>, _> = packets
            .into_iter()
            .map(|mix_packet| self.mix_packet_to_ws_message(mix_packet))
            .collect();

        if let Err(err) = self
            .batch_send_websocket_messages_without_response(messages?)
            .await
        {
            if err.is_closed_connection() && self.cfg.connection.should_reconnect_on_failure {
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
            if err.is_closed_connection() && self.cfg.connection.should_reconnect_on_failure {
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
        let bandwidth_remaining = self.bandwidth.remaining();
        if bandwidth_remaining < self.cfg.bandwidth.remaining_bandwidth_threshold {
            self.cfg
                .bandwidth
                .ensure_above_cutoff(bandwidth_remaining)?;
            self.claim_bandwidth().await?;
        }

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        let msg = self.mix_packet_to_ws_message(mix_packet)?;
        self.send_with_reconnection_on_failure(msg).await
    }

    // SAFETY: this method is only called when the connection is in `PartiallyDelegated` state
    #[allow(clippy::unreachable)]
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

        #[allow(clippy::expect_used)]
        let partially_delegated =
            match std::mem::replace(&mut self.connection, SocketState::Invalid) {
                SocketState::Available(conn) => {
                    PartiallyDelegatedHandle::split_and_listen_for_mixnet_messages(
                        *conn,
                        self.packet_router.clone(),
                        Arc::clone(
                            self.shared_key
                                .as_ref()
                                .expect("no shared key present even though we're authenticated!"),
                        ),
                        self.bandwidth.clone(),
                        self.shutdown_token.clone(),
                    )
                }
                other => {
                    error!(
                        "attempted to start mixnet listener whilst the connection is in {} state!",
                        other.name()
                    );
                    return Err(GatewayClientError::ConnectionInInvalidState);
                }
            };

        self.connection = SocketState::PartiallyDelegated(partially_delegated);
        Ok(())
    }

    pub async fn try_reconnect(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }

        // if we're reconnecting, because we lost connection, we need to re-authenticate the connection
        if let Some(negotiated_protocol) = self.negotiated_protocol {
            self.authenticate(negotiated_protocol).await?;
        } else {
            // This should never happen, because it would mean we're not registered
            return Err(GatewayClientError::NotRegistered);
        }

        // this call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), GatewayClientError> {
        self.recover_socket_connection().await?;
        self.connection = SocketState::NotConnected;
        Ok(())
    }

    pub async fn claim_initial_bandwidth(&mut self) -> Result<(), GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }

        let bandwidth_remaining = self.bandwidth.remaining();
        if bandwidth_remaining < self.cfg.bandwidth.remaining_bandwidth_threshold {
            self.cfg
                .bandwidth
                .ensure_above_cutoff(bandwidth_remaining)?;
            info!("Claiming more bandwidth with existing credentials. Stop the process now if you don't want that to happen.");
            self.claim_bandwidth().await?;
        }
        Ok(())
    }
}

// type alias for an ease of use
pub type InitGatewayClient = GatewayClient<InitOnly>;

#[derive(Debug)]
pub struct InitOnly;

impl GatewayClient<InitOnly, EphemeralCredentialStorage> {
    // for initialisation we do not need credential storage. Though it's still a bit weird we have to set the generic...
    pub fn new_init(
        gateway_details: EntryDetails,
        gateway_identity: ed25519::PublicKey,
        local_identity: Arc<ed25519::KeyPair>,
        #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
    ) -> Self {
        log::trace!("Initialising gateway client");
        use futures::channel::mpsc;

        // note: this packet_router is completely invalid in normal circumstances, but "works"
        // perfectly fine here, because it's not meant to be used
        let (ack_tx, _) = mpsc::unbounded();
        let (mix_tx, _) = mpsc::unbounded();
        let shutdown_token = ShutdownToken::default();
        let packet_router = PacketRouter::new(ack_tx, mix_tx, shutdown_token.clone());

        GatewayClient {
            cfg: GatewayClientConfig::default().with_disabled_credentials_mode(true),
            authenticated: false,
            bandwidth: ClientBandwidth::new_empty(),
            gateway_details,
            gateway_identity,
            local_identity,
            shared_key: None,
            connection: SocketState::NotConnected,
            packet_router,
            bandwidth_controller: None,
            stats_reporter: ClientStatsSender::new(None, shutdown_token.clone()),
            negotiated_protocol: None,
            #[cfg(unix)]
            connection_fd_callback,
            shutdown_token,
        }
    }

    pub fn upgrade<C, St>(
        self,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        stats_reporter: ClientStatsSender,
        shutdown_token: ShutdownToken,
    ) -> GatewayClient<C, St> {
        // invariants that can't be broken
        // (unless somebody decided to expose some field that wasn't meant to be exposed)
        assert!(self.authenticated);
        assert!(self.connection.is_available());
        assert!(self.shared_key.is_some());

        GatewayClient {
            cfg: self.cfg,
            authenticated: self.authenticated,
            bandwidth: self.bandwidth,
            gateway_details: self.gateway_details,
            gateway_identity: self.gateway_identity,
            local_identity: self.local_identity,
            shared_key: self.shared_key,
            connection: self.connection,
            packet_router,
            bandwidth_controller,
            stats_reporter,
            negotiated_protocol: self.negotiated_protocol,
            #[cfg(unix)]
            connection_fd_callback: self.connection_fd_callback,
            shutdown_token,
        }
    }
}
