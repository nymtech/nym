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

use crate::cleanup_socket_message;
use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
pub use crate::packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use crate::socket_state::{PartiallyDelegated, SocketState};
use crypto::asymmetric::identity;
use futures::{FutureExt, SinkExt, StreamExt};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::authentication::iv::AuthenticationIV;
use gateway_requests::registration::handshake::{client_handshake, SharedKeys, DEFAULT_RNG};
use gateway_requests::{BinaryRequest, ClientControlRequest, ServerResponse};
use nymsphinx::forwarding::packet::MixPacket;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use tungstenite::protocol::Message;

#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::connect_async;

#[cfg(target_arch = "wasm32")]
use wasm_timer;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

pub struct GatewayClient {
    authenticated: bool,
    // can be String, string slices, `url::Url`, `http::Uri`, etc.
    gateway_address: String,
    gateway_identity: identity::PublicKey,
    local_identity: Arc<identity::KeyPair>,
    shared_key: Option<Arc<SharedKeys>>,
    connection: SocketState,
    packet_router: PacketRouter,
    response_timeout_duration: Duration,
}

impl GatewayClient {
    // TODO: put it all in a Config struct
    pub fn new(
        gateway_address: String,
        local_identity: Arc<identity::KeyPair>,
        gateway_identity: identity::PublicKey,
        shared_key: Option<Arc<SharedKeys>>,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        response_timeout_duration: Duration,
    ) -> Self {
        GatewayClient {
            authenticated: false,
            gateway_address,
            gateway_identity,
            local_identity,
            shared_key,
            connection: SocketState::NotConnected,
            packet_router: PacketRouter::new(ack_sender, mixnet_message_sender),
            response_timeout_duration,
        }
    }

    pub fn new_init(
        gateway_address: String,
        gateway_identity: identity::PublicKey,
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
            gateway_address,
            gateway_identity,
            local_identity,
            shared_key: None,
            connection: SocketState::NotConnected,
            packet_router,
            response_timeout_duration,
        }
    }

    pub fn identity(&self) -> identity::PublicKey {
        self.gateway_identity
    }

    pub async fn close_connection(&mut self) -> Result<(), GatewayClientError> {
        if self.connection.is_partially_delegated() {
            self.recover_socket_connection().await?;
        }

        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            #[cfg(not(target_arch = "wasm32"))]
            SocketState::Available(mut socket) => Ok(socket.close(None).await?),
            #[cfg(target_arch = "wasm32")]
            SocketState::Available(mut socket) => Ok(socket.close(None).await),
            SocketState::PartiallyDelegated(_) => {
                unreachable!("this branch should have never been reached!")
            }
            _ => Ok(()), // no need to do anything in those cases
        }
    }

    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        let ws_stream = match connect_async(&self.gateway_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(GatewayClientError::NetworkError(e)),
        };

        #[cfg(target_arch = "wasm32")]
        let ws_stream = match JSWebsocket::new(&self.gateway_address) {
            Ok(ws_stream) => ws_stream,
            Err(e) => return Err(GatewayClientError::NetworkErrorWasm(e)),
        };

        self.connection = SocketState::Available(ws_stream);
        Ok(())
    }

    async fn read_control_response(&mut self) -> Result<ServerResponse, GatewayClientError> {
        // we use the fact that all request responses are Message::Text and only pushed
        // sphinx packets are Message::Binary

        let conn = match self.connection {
            SocketState::Available(ref mut conn) => conn,
            _ => return Err(GatewayClientError::ConnectionInInvalidState),
        };

        #[cfg(not(target_arch = "wasm32"))]
        let timeout = tokio::time::delay_for(self.response_timeout_duration);

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
                partially_delegated
                    .batch_send_without_response(messages)
                    .await
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
                partially_delegated.send_without_response(msg).await
            }
            SocketState::NotConnected => Err(GatewayClientError::ConnectionNotEstablished),
            _ => Err(GatewayClientError::ConnectionInInvalidState),
        }
    }

    pub async fn register(&mut self) -> Result<SharedKeys, GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }

        debug_assert!(self.connection.is_available());

        let shared_key = match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut DEFAULT_RNG,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => unreachable!(),
        }?;

        self.authenticated = true;
        Ok(shared_key)
    }

    pub async fn authenticate(
        &mut self,
        shared_key: Option<SharedKeys>,
    ) -> Result<bool, GatewayClientError> {
        if shared_key.is_none() && self.shared_key.is_none() {
            return Err(GatewayClientError::NoSharedKeyAvailable);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        // because of the previous check one of the unwraps MUST succeed
        let shared_key = shared_key
            .as_ref()
            .unwrap_or_else(|| self.shared_key.as_ref().unwrap());
        let iv = AuthenticationIV::new_random(&mut DEFAULT_RNG);
        let self_address = self
            .local_identity
            .as_ref()
            .public_key()
            .derive_destination_address();
        let encrypted_address = EncryptedAddressBytes::new(&self_address, shared_key, &iv);

        let msg =
            ClientControlRequest::new_authenticate(self_address, encrypted_address, iv).into();

        let authenticated = match self.send_websocket_message(msg).await? {
            ServerResponse::Authenticate { status } => {
                self.authenticated = status;
                Ok(status)
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => unreachable!(),
        }?;
        Ok(authenticated)
    }

    /// Helper method to either call register or authenticate based on self.shared_key value
    pub async fn perform_initial_authentication(
        &mut self,
    ) -> Result<Arc<SharedKeys>, GatewayClientError> {
        if self.shared_key.is_some() {
            self.authenticate(None).await?;
        } else {
            let shared_key = self.register().await?;
            self.shared_key = Some(Arc::new(shared_key));
        }
        if self.authenticated {
            // if we are authenticated it means we MUST have an associated shared_key
            Ok(Arc::clone(&self.shared_key.as_ref().unwrap()))
        } else {
            Err(GatewayClientError::AuthenticationFailure)
        }
    }

    pub async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
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

        self.batch_send_websocket_messages_without_response(messages)
            .await
    }

    // TODO: possibly make responses optional
    pub async fn send_mix_packet(
        &mut self,
        mix_packet: MixPacket,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
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
        self.send_websocket_message_without_response(msg).await
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

        self.connection = SocketState::Available(conn);
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
                        conn,
                        self.packet_router.clone(),
                        Arc::clone(
                            self.shared_key
                                .as_ref()
                                .expect("no shared key present even though we're authenticated!"),
                        ),
                    )?
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

        // this call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(shared_key)
    }
}
