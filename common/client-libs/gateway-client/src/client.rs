// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bandwidth::BandwidthController;
use crate::cleanup_socket_message;
use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
pub use crate::packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use crate::socket_state::{PartiallyDelegated, SocketState};
#[cfg(target_arch = "wasm32")]
use crate::wasm_storage::PersistentStorage;
#[cfg(feature = "coconut")]
use coconut_interface::Credential;
#[cfg(not(target_arch = "wasm32"))]
use credential_storage::PersistentStorage;
#[cfg(not(feature = "coconut"))]
use credentials::token::bandwidth::TokenCredential;
use crypto::asymmetric::identity;
use futures::{FutureExt, SinkExt, StreamExt};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::iv::IV;
use gateway_requests::registration::handshake::{client_handshake, SharedKeys};
use gateway_requests::{BinaryRequest, ClientControlRequest, ServerResponse};
use log::*;
use network_defaults::{REMAINING_BANDWIDTH_THRESHOLD, TOKENS_TO_BURN};
use nymsphinx::forwarding::packet::MixPacket;
use rand::rngs::OsRng;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use tungstenite::protocol::Message;

#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::connect_async;

#[cfg(target_arch = "wasm32")]
use fluvio_wasm_timer as wasm_timer;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

const DEFAULT_RECONNECTION_ATTEMPTS: usize = 10;
const DEFAULT_RECONNECTION_BACKOFF: Duration = Duration::from_secs(5);

pub struct GatewayClient {
    authenticated: bool,
    testnet_mode: bool,
    bandwidth_remaining: i64,
    gateway_address: String,
    gateway_identity: identity::PublicKey,
    gateway_owner: String,
    local_identity: Arc<identity::KeyPair>,
    shared_key: Option<Arc<SharedKeys>>,
    connection: SocketState,
    packet_router: PacketRouter,
    response_timeout_duration: Duration,
    bandwidth_controller: Option<BandwidthController<PersistentStorage>>,

    // reconnection related variables
    /// Specifies whether client should try to reconnect to gateway on connection failure.
    should_reconnect_on_failure: bool,
    /// Specifies maximum number of attempts client will try to reconnect to gateway on failure
    /// before giving up.
    reconnection_attempts: usize,
    /// Delay between each subsequent reconnection attempt.
    reconnection_backoff: Duration,
}

impl GatewayClient {
    // TODO: put it all in a Config struct
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gateway_address: String,
        local_identity: Arc<identity::KeyPair>,
        gateway_identity: identity::PublicKey,
        gateway_owner: String,
        shared_key: Option<Arc<SharedKeys>>,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        response_timeout_duration: Duration,
        bandwidth_controller: Option<BandwidthController<PersistentStorage>>,
    ) -> Self {
        GatewayClient {
            authenticated: false,
            testnet_mode: false,
            bandwidth_remaining: 0,
            gateway_address,
            gateway_identity,
            gateway_owner,
            local_identity,
            shared_key,
            connection: SocketState::NotConnected,
            packet_router: PacketRouter::new(ack_sender, mixnet_message_sender),
            response_timeout_duration,
            bandwidth_controller,
            should_reconnect_on_failure: true,
            reconnection_attempts: DEFAULT_RECONNECTION_ATTEMPTS,
            reconnection_backoff: DEFAULT_RECONNECTION_BACKOFF,
        }
    }

    pub fn set_testnet_mode(&mut self, testnet_mode: bool) {
        self.testnet_mode = testnet_mode
    }

    // TODO: later convert into proper builder methods
    pub fn with_reconnection_on_failure(&mut self, should_reconnect_on_failure: bool) {
        self.should_reconnect_on_failure = should_reconnect_on_failure
    }

    pub fn with_reconnection_attempts(&mut self, reconnection_attempts: usize) {
        self.reconnection_attempts = reconnection_attempts
    }

    pub fn with_reconnection_backoff(&mut self, backoff: Duration) {
        self.reconnection_backoff = backoff
    }

    pub fn new_init(
        gateway_address: String,
        gateway_identity: identity::PublicKey,
        gateway_owner: String,
        local_identity: Arc<identity::KeyPair>,
        response_timeout_duration: Duration,
    ) -> Self {
        use futures::channel::mpsc;

        // note: this packet_router is completely invalid in normal circumstances, but "works"
        // perfectly fine here, because it's not meant to be used
        let (ack_tx, _) = mpsc::unbounded();
        let (mix_tx, _) = mpsc::unbounded();
        let packet_router = PacketRouter::new(ack_tx, mix_tx);

        GatewayClient {
            authenticated: false,
            testnet_mode: false,
            bandwidth_remaining: 0,
            gateway_address,
            gateway_identity,
            gateway_owner,
            local_identity,
            shared_key: None,
            connection: SocketState::NotConnected,
            packet_router,
            response_timeout_duration,
            bandwidth_controller: None,
            should_reconnect_on_failure: false,
            reconnection_attempts: DEFAULT_RECONNECTION_ATTEMPTS,
            reconnection_backoff: DEFAULT_RECONNECTION_BACKOFF,
        }
    }

    pub fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway_identity
    }

    pub fn remaining_bandwidth(&self) -> i64 {
        self.bandwidth_remaining
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
            SocketState::Available(mut socket) => {
                (*socket).close(None).await;
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
        let ws_stream = match connect_async(&self.gateway_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(GatewayClientError::NetworkError(e)),
        };

        self.connection = SocketState::Available(Box::new(ws_stream));
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
        let ws_stream = match JSWebsocket::new(&self.gateway_address) {
            Ok(ws_stream) => ws_stream,
            Err(e) => return Err(GatewayClientError::NetworkErrorWasm(e)),
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
            if self.authenticate_and_start().await.is_ok() {
                info!("managed to reconnect!");
                return Ok(());
            }

            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(self.reconnection_backoff).await;

            #[cfg(target_arch = "wasm32")]
            if let Err(err) = wasm_timer::Delay::new(self.reconnection_backoff).await {
                error!(
                    "the timer has gone away while in reconnection backoff! - {}",
                    err
                );
            }
        }

        // final attempt (done separately to be able to return a proper error)
        info!("attempt {}", self.reconnection_attempts);
        match self.authenticate_and_start().await {
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

        #[cfg(not(target_arch = "wasm32"))]
        let timeout = tokio::time::sleep(self.response_timeout_duration);
        #[cfg(not(target_arch = "wasm32"))]
        tokio::pin!(timeout);

        // technically the `wasm_timer` also works outside wasm, but unless required,
        // I really prefer to just stick to tokio
        #[cfg(target_arch = "wasm32")]
        let timeout = wasm_timer::Delay::new(self.response_timeout_duration);

        let mut fused_timeout = timeout.fuse();
        let mut fused_stream = conn.fuse();

        loop {
            futures::select! {
                _ = &mut fused_timeout => {
                    break Err(GatewayClientError::Timeout);
                }
                msg = fused_stream.next() => {
                    let ws_msg = match cleanup_socket_message(msg) {
                        Err(err) => break Err(err),
                        Ok(msg) => msg
                    };
                    match ws_msg {
                        Message::Binary(bin_msg) => {
                            self.packet_router.route_received(vec![bin_msg]);
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
                    error!("failed to batch send messages - {}...", err);
                    // we must ensure we do not leave the task still active
                    if let Err(err) = self.recover_socket_connection().await {
                        error!(
                            "... and the delegated stream has also errored out - {}",
                            err
                        )
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
                    error!("failed to send message without response - {}...", err);
                    // we must ensure we do not leave the task still active
                    if let Err(err) = self.recover_socket_connection().await {
                        error!(
                            "... and the delegated stream has also errored out - {}",
                            err
                        )
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

    async fn register(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        debug_assert!(self.connection.is_available());

        // it's fine to instantiate it here as it's only used once (during authentication or registration)
        // and putting it into the GatewayClient struct would be a hassle
        let mut rng = OsRng;

        let shared_key = match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut rng,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => unreachable!(),
        }?;
        self.authenticated = match self.read_control_response().await? {
            ServerResponse::Register { status } => Ok(status),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;
        if self.authenticated {
            self.shared_key = Some(Arc::new(shared_key));
        }
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

        let msg =
            ClientControlRequest::new_authenticate(self_address, encrypted_address, iv).into();

        match self.send_websocket_message(msg).await? {
            ServerResponse::Authenticate {
                status,
                bandwidth_remaining,
            } => {
                self.authenticated = status;
                self.bandwidth_remaining = bandwidth_remaining;
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

    #[cfg(feature = "coconut")]
    async fn claim_coconut_bandwidth(
        &mut self,
        credential: Credential,
    ) -> Result<(), GatewayClientError> {
        let mut rng = OsRng;
        let iv = IV::new_random(&mut rng);

        let msg = ClientControlRequest::new_enc_coconut_bandwidth_credential(
            &credential,
            self.shared_key.as_ref().unwrap(),
            iv,
        )
        .ok_or(GatewayClientError::SerializeCredential)?
        .into();
        self.bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;
        Ok(())
    }

    #[cfg(not(feature = "coconut"))]
    async fn claim_token_bandwidth(
        &mut self,
        credential: TokenCredential,
    ) -> Result<(), GatewayClientError> {
        let mut rng = OsRng;

        let iv = IV::new_random(&mut rng);

        let msg = ClientControlRequest::new_enc_token_bandwidth_credential(
            &credential,
            self.shared_key.as_ref().unwrap(),
            iv,
        )
        .into();
        self.bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;

        Ok(())
    }

    async fn try_claim_testnet_bandwidth(&mut self) -> Result<(), GatewayClientError> {
        let msg = ClientControlRequest::ClaimFreeTestnetBandwidth.into();
        self.bandwidth_remaining = match self.send_websocket_message(msg).await? {
            ServerResponse::Bandwidth { available_total } => Ok(available_total),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => Err(GatewayClientError::UnexpectedResponse),
        }?;

        Ok(())
    }

    pub async fn claim_bandwidth(&mut self) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if self.bandwidth_controller.is_none() && !self.testnet_mode {
            return Err(GatewayClientError::NoBandwidthControllerAvailable);
        }

        warn!("Not enough bandwidth. Trying to get more bandwidth, this might take a while");
        if self.testnet_mode {
            info!("The client is running in testnet mode - attempting to claim bandwidth without a credential");
            return self.try_claim_testnet_bandwidth().await;
        }

        let _gateway_owner = self.gateway_owner.clone();

        #[cfg(feature = "coconut")]
        let credential = self
            .bandwidth_controller
            .as_ref()
            .unwrap()
            .prepare_coconut_credential()
            .await?;
        #[cfg(not(feature = "coconut"))]
        let credential = self
            .bandwidth_controller
            .as_ref()
            .unwrap()
            .prepare_token_credential(self.gateway_identity, _gateway_owner)
            .await?;

        #[cfg(feature = "coconut")]
        return self.claim_coconut_bandwidth(credential).await;
        #[cfg(not(feature = "coconut"))]
        return self.claim_token_bandwidth(credential).await;
    }

    fn estimate_required_bandwidth(&self, packets: &[MixPacket]) -> i64 {
        packets
            .iter()
            .map(|packet| packet.sphinx_packet().len())
            .sum::<usize>() as i64
    }

    pub async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if self.estimate_required_bandwidth(&packets) > self.bandwidth_remaining {
            return Err(GatewayClientError::NotEnoughBandwidth(
                self.estimate_required_bandwidth(&packets),
                self.bandwidth_remaining,
            ));
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
                info!("Going to attempt a reconnection");
                self.attempt_reconnection().await
            } else {
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
    pub async fn send_mix_packet(
        &mut self,
        mix_packet: MixPacket,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if (mix_packet.sphinx_packet().len() as i64) > self.bandwidth_remaining {
            return Err(GatewayClientError::NotEnoughBandwidth(
                mix_packet.sphinx_packet().len() as i64,
                self.bandwidth_remaining,
            ));
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
                    )
                }
                _ => unreachable!(),
            };

        self.connection = SocketState::PartiallyDelegated(partially_delegated);
        Ok(())
    }

    pub async fn authenticate_and_start(&mut self) -> Result<Arc<SharedKeys>, GatewayClientError> {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }
        let shared_key = self.perform_initial_authentication().await?;

        if self.bandwidth_remaining < REMAINING_BANDWIDTH_THRESHOLD {
            info!("Claiming more bandwidth for your tokens. This will use {} token(s) from your wallet. \
            Stop the process now if you don't want that to happen.", TOKENS_TO_BURN);
            self.claim_bandwidth().await?;
        }

        // this call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(shared_key)
    }
}
