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
use nym_bandwidth_controller::{BandwidthController, BandwidthStatusMessage};
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_credentials::CredentialSpendingData;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::identity;
use nym_gateway_requests::registration::handshake::client_handshake;
use nym_gateway_requests::{
    BinaryRequest, ClientControlRequest, ClientRequest, SensitiveServerResponse, ServerResponse,
    SharedGatewayKey, SharedSymmetricKey, AES_GCM_SIV_PROTOCOL_VERSION,
    CREDENTIAL_UPDATE_V2_PROTOCOL_VERSION, CURRENT_PROTOCOL_VERSION,
};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_task::TaskClient;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use rand::rngs::OsRng;
use std::sync::Arc;
use tracing::instrument;
use tracing::*;
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
use zeroize::Zeroizing;

pub mod config;

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

#[must_use]
#[derive(Debug)]
pub struct AuthenticationResponse {
    pub initial_shared_key: Arc<SharedGatewayKey>,
    pub requires_key_upgrade: bool,
}

// TODO: this should be refactored into a state machine that keeps track of its authentication state
pub struct GatewayClient<C, St = EphemeralCredentialStorage> {
    pub cfg: GatewayClientConfig,

    authenticated: bool,
    bandwidth: ClientBandwidth,
    gateway_address: String,
    gateway_identity: identity::PublicKey,
    local_identity: Arc<identity::KeyPair>,
    shared_key: Option<Arc<SharedGatewayKey>>,
    connection: SocketState,
    packet_router: PacketRouter,
    bandwidth_controller: Option<BandwidthController<C, St>>,

    // currently unused (but populated)
    negotiated_protocol: Option<u8>,

    /// Listen to shutdown messages and send notifications back to the task manager
    task_client: TaskClient,
}

impl<C, St> GatewayClient<C, St> {
    pub fn new(
        cfg: GatewayClientConfig,
        gateway_config: GatewayConfig,
        local_identity: Arc<identity::KeyPair>,
        // TODO: make it mandatory. if you don't want to pass it, use `new_init`
        shared_key: Option<Arc<SharedGatewayKey>>,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        task_client: TaskClient,
    ) -> Self {
        GatewayClient {
            cfg,
            authenticated: false,
            bandwidth: ClientBandwidth::new_empty(),
            gateway_address: gateway_config.gateway_listener,
            gateway_identity: gateway_config.gateway_identity,
            local_identity,
            shared_key,
            connection: SocketState::NotConnected,
            packet_router,
            bandwidth_controller,
            negotiated_protocol: None,
            task_client,
        }
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
        self.bandwidth.remaining()
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
                _ = self.task_client.recv() => {
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

    async fn register(
        &mut self,
        derive_aes256_gcm_siv_key: bool,
    ) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        debug_assert!(self.connection.is_available());
        log::debug!(
            "registering with gateway. using legacy key derivation: {}",
            !derive_aes256_gcm_siv_key
        );

        // it's fine to instantiate it here as it's only used once (during authentication or registration)
        // and putting it into the GatewayClient struct would be a hassle
        let mut rng = OsRng;

        let shared_key = match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut rng,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
                self.cfg.bandwidth.require_tickets,
                derive_aes256_gcm_siv_key,
                #[cfg(not(target_arch = "wasm32"))]
                self.task_client.clone(),
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        }?;

        let (authentication_status, gateway_protocol) = match self.read_control_response().await? {
            ServerResponse::Register {
                protocol_version,
                status,
            } => (status, protocol_version),
            ServerResponse::Error { message } => {
                return Err(GatewayClientError::GatewayError(message))
            }
            other => return Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
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

    pub async fn upgrade_key_authenticated(
        &mut self,
    ) -> Result<Zeroizing<SharedSymmetricKey>, GatewayClientError> {
        info!("*** STARTING AES128CTR-HMAC KEY UPGRADE INTO AES256GCM-SIV***");

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }

        let Some(shared_key) = self.shared_key.as_ref() else {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        };

        if !shared_key.is_legacy() {
            return Err(GatewayClientError::KeyAlreadyUpgraded);
        }

        // make sure we have the only reference, so we could safely swap it
        if Arc::strong_count(shared_key) != 1 {
            return Err(GatewayClientError::KeyAlreadyInUse);
        }

        assert!(shared_key.is_legacy());
        let legacy_key = shared_key.unwrap_legacy();
        let (updated_key, hkdf_salt) = legacy_key.upgrade();
        let derived_key_digest = updated_key.digest();

        let upgrade_request = ClientRequest::UpgradeKey {
            hkdf_salt,
            derived_key_digest,
        }
        .encrypt(legacy_key)?;

        info!("sending upgrade request and awaiting the acknowledgement back");
        let (ciphertext, nonce) = match self.send_websocket_message(upgrade_request).await? {
            ServerResponse::EncryptedResponse { ciphertext, nonce } => (ciphertext, nonce),
            ServerResponse::Error { message } => {
                return Err(GatewayClientError::GatewayError(message))
            }
            other => return Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        };

        // attempt to decrypt it using NEW key
        let Ok(response) = SensitiveServerResponse::decrypt(&ciphertext, &nonce, &updated_key)
        else {
            return Err(GatewayClientError::FatalKeyUpgradeFailure);
        };

        match response {
            SensitiveServerResponse::KeyUpgradeAck { .. } => {
                info!("received key upgrade acknowledgement")
            }
            _ => return Err(GatewayClientError::FatalKeyUpgradeFailure),
        }

        // perform in memory swap and make a copy for updating storage
        let zeroizing_updated_key = updated_key.zeroizing_clone();
        self.shared_key = Some(Arc::new(updated_key.into()));

        Ok(zeroizing_updated_key)
    }

    async fn authenticate(&mut self) -> Result<(), GatewayClientError> {
        let Some(shared_key) = self.shared_key.as_ref() else {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        };

        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        debug!("authenticating with gateway");

        let self_address = self
            .local_identity
            .as_ref()
            .public_key()
            .derive_destination_address();

        let msg = ClientControlRequest::new_authenticate(
            self_address,
            shared_key,
            self.cfg.bandwidth.require_tickets,
        )?;

        match self.send_websocket_message(msg).await? {
            ServerResponse::Authenticate {
                protocol_version,
                status,
                bandwidth_remaining,
            } => {
                self.check_gateway_protocol(protocol_version)?;
                self.authenticated = status;
                self.bandwidth.update_and_maybe_log(bandwidth_remaining);

                self.negotiated_protocol = protocol_version;
                log::debug!("authenticated: {status}, bandwidth remaining: {bandwidth_remaining}");

                self.task_client.send_status_msg(Box::new(
                    BandwidthStatusMessage::RemainingBandwidth(bandwidth_remaining),
                ));
                Ok(())
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }
    }

    /// Helper method to either call register or authenticate based on self.shared_key value
    #[instrument(skip_all,
        fields(
            gateway = %self.gateway_identity,
            gateway_address = %self.gateway_address
        )
    )]
    pub async fn perform_initial_authentication(
        &mut self,
    ) -> Result<AuthenticationResponse, GatewayClientError> {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }

        // 1. check gateway's protocol version
        let supports_aes_gcm_siv = match self.get_gateway_protocol().await {
            Ok(protocol) => protocol >= AES_GCM_SIV_PROTOCOL_VERSION,
            Err(_) => {
                // if we failed to send the request, it means the gateway is running the old binary,
                // so it has reset our connection - we have to reconnect
                self.establish_connection().await?;
                false
            }
        };

        if !supports_aes_gcm_siv {
            warn!("this gateway is on an old version that doesn't support AES256-GCM-SIV");
        }

        if self.authenticated {
            debug!("Already authenticated");
            return if let Some(shared_key) = &self.shared_key {
                Ok(AuthenticationResponse {
                    initial_shared_key: Arc::clone(shared_key),
                    requires_key_upgrade: shared_key.is_legacy() && supports_aes_gcm_siv,
                })
            } else {
                Err(GatewayClientError::AuthenticationFailureWithPreexistingSharedKey)
            };
        }

        if self.shared_key.is_some() {
            self.authenticate().await?;

            if self.authenticated {
                // if we are authenticated it means we MUST have an associated shared_key
                let shared_key = self.shared_key.as_ref().unwrap();

                let requires_key_upgrade = shared_key.is_legacy() && supports_aes_gcm_siv;

                Ok(AuthenticationResponse {
                    initial_shared_key: Arc::clone(shared_key),
                    requires_key_upgrade,
                })
            } else {
                Err(GatewayClientError::AuthenticationFailure)
            }
        } else {
            self.register(supports_aes_gcm_siv).await?;

            // if registration didn't return an error, we MUST have an associated shared key
            let shared_key = self.shared_key.as_ref().unwrap();

            // we're always registering with the highest supported protocol,
            // so no upgrades are required
            Ok(AuthenticationResponse {
                initial_shared_key: Arc::clone(shared_key),
                requires_key_upgrade: false,
            })
        }
    }

    pub async fn get_gateway_protocol(&mut self) -> Result<u8, GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        match self
            .send_websocket_message(ClientControlRequest::SupportedProtocol {})
            .await?
        {
            ServerResponse::SupportedProtocol { version } => Ok(version),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }
    }

    async fn claim_ecash_bandwidth(
        &mut self,
        credential: CredentialSpendingData,
    ) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::new_enc_ecash_credential(
            credential,
            self.shared_key.as_ref().unwrap(),
        )?;
        let bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            ServerResponse::TypedError { error } => {
                Err(GatewayClientError::TypedGatewayError(error))
            }
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }?;

        // TODO: create tracing span
        info!("managed to claim ecash bandwidth");
        self.bandwidth.update_and_log(bandwidth_remaining);

        Ok(())
    }

    async fn try_claim_testnet_bandwidth(&mut self) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::ClaimFreeTestnetBandwidth;
        let bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            other => Err(GatewayClientError::UnexpectedResponse { name: other.name() }),
        }?;

        info!("managed to claim testnet bandwidth");
        self.bandwidth.update_and_log(bandwidth_remaining);

        Ok(())
    }

    fn unchecked_bandwidth_controller(&self) -> &BandwidthController<C, St> {
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

        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if self.bandwidth_controller.is_none() && self.cfg.bandwidth.require_tickets {
            return Err(GatewayClientError::NoBandwidthControllerAvailable);
        }

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
                TicketType::V1MixnetEntry,
                self.gateway_identity.to_bytes(),
                TICKETS_TO_SPEND,
            )
            .await?;

        match self.claim_ecash_bandwidth(prepared_credential.data).await {
            Ok(_) => Ok(()),
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
            .map(|mix_packet| {
                BinaryRequest::ForwardSphinx { packet: mix_packet }.into_ws_message(
                    self.shared_key
                        .as_ref()
                        .expect("no shared key present even though we're authenticated!"),
                )
            })
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
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let msg = BinaryRequest::ForwardSphinx { packet: mix_packet }.into_ws_message(
            self.shared_key
                .as_ref()
                .expect("no shared key present even though we're authenticated!"),
        )?;
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
                    PartiallyDelegatedHandle::split_and_listen_for_mixnet_messages(
                        *conn,
                        self.packet_router.clone(),
                        Arc::clone(
                            self.shared_key
                                .as_ref()
                                .expect("no shared key present even though we're authenticated!"),
                        ),
                        self.bandwidth.clone(),
                        self.task_client.clone(),
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

        // if we're reconnecting, because we lost connection, we need to re-authenticate the connection
        self.authenticate().await?;

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

    #[deprecated(note = "this method does not deal with upgraded keys for legacy clients")]
    pub async fn authenticate_and_start(
        &mut self,
    ) -> Result<AuthenticationResponse, GatewayClientError>
    where
        C: DkgQueryClient + Send + Sync,
        St: CredentialStorage,
        <St as CredentialStorage>::StorageError: Send + Sync + 'static,
    {
        let shared_key = self.perform_initial_authentication().await?;
        self.claim_initial_bandwidth().await?;

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
        let task_client = TaskClient::dummy();
        let packet_router = PacketRouter::new(ack_tx, mix_tx, task_client.clone());

        GatewayClient {
            cfg: GatewayClientConfig::default().with_disabled_credentials_mode(true),
            authenticated: false,
            bandwidth: ClientBandwidth::new_empty(),
            gateway_address: gateway_listener.to_string(),
            gateway_identity,
            local_identity,
            shared_key: None,
            connection: SocketState::NotConnected,
            packet_router,
            bandwidth_controller: None,
            negotiated_protocol: None,
            task_client,
        }
    }

    pub fn upgrade<C, St>(
        self,
        packet_router: PacketRouter,
        bandwidth_controller: Option<BandwidthController<C, St>>,
        task_client: TaskClient,
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
            gateway_address: self.gateway_address,
            gateway_identity: self.gateway_identity,
            local_identity: self.local_identity,
            shared_key: self.shared_key,
            connection: self.connection,
            packet_router,
            bandwidth_controller,
            negotiated_protocol: self.negotiated_protocol,
            task_client,
        }
    }
}
