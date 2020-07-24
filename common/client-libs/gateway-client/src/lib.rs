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

use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
pub use crate::packet_router::{
    AcknowledgementReceiver, AcknowledgementSender, MixnetMessageReceiver, MixnetMessageSender,
};
use crypto::asymmetric::identity;
use futures::stream::{SplitSink, SplitStream};
use futures::{future::BoxFuture, FutureExt, SinkExt, Stream, StreamExt};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::authentication::iv::AuthenticationIV;
use gateway_requests::registration::handshake::{client_handshake, SharedKeys, DEFAULT_RNG};
use gateway_requests::{BinaryRequest, ClientControlRequest, ServerResponse};
use log::*;
use nymsphinx::{addressing::nodes::NymNodeRoutingAddress, SphinxPacket};
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message, Error as WsError},
    WebSocketStream,
};

pub mod error;
pub mod packet_router;

/// A helper method to read an underlying message from the stream or return an error.
async fn read_ws_stream_message<S>(conn: &mut S) -> Result<Message, GatewayClientError>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    match conn.next().await {
        Some(msg) => match msg {
            Ok(msg) => Ok(msg),
            Err(err) => Err(GatewayClientError::NetworkError(err)),
        },
        None => Err(GatewayClientError::ConnectionAbruptlyClosed),
    }
}

// TODO: some batching mechanism to allow reading and sending more than a single packet through

// type alias for not having to type the whole thing every single time
type WsConn = WebSocketStream<TcpStream>;

// We have ownership over sink half of the connection, but the stream is owned
// by some other task, however, we can notify it to get the stream back.
struct PartiallyDelegated<'a> {
    sink_half: SplitSink<WsConn, Message>,
    delegated_stream: (
        BoxFuture<'a, Result<SplitStream<WsConn>, GatewayClientError>>,
        Arc<Notify>,
    ),
}

impl<'a> PartiallyDelegated<'a> {
    // TODO: this can be potentially bad as we have no direct restrictions of ensuring it's called
    // within tokio runtime. Perhaps we should use the "old" way of passing explicit
    // runtime handle to the constructor and using that instead?
    fn split_and_listen_for_mixnet_messages(
        conn: WsConn,
        packet_router: PacketRouter,
    ) -> Result<Self, GatewayClientError> {
        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        let notify = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&notify);

        let (sink, mut stream) = conn.split();

        let mixnet_receiver_future = async move {
            let mut should_return = false;
            while !should_return {
                tokio::select! {
                    _ = notify_clone.notified() => {
                        should_return = true;
                    }
                    msg = read_ws_stream_message(&mut stream) => {
                        match msg? {
                            Message::Binary(bin_msg) => {
                                // TODO: some batching mechanism to allow reading and sending more than
                                // one packet at the time, because the receiver can easily handle it
                                packet_router.route_received(vec![bin_msg])
                            },
                            // I think that in the future we should perhaps have some sequence number system, i.e.
                            // so each request/response pair can be easily identified, so that if messages are
                            // not ordered (for some peculiar reason) we wouldn't lose anything.
                            // This would also require NOT discarding any text responses here.
                            Message::Text(_) => debug!("received a text message - probably a response to some previous query!"),
                            _ => (),
                        };
                    }
                };
            }
            Ok(stream)
        };

        let spawned_boxed_task = tokio::spawn(mixnet_receiver_future)
            .map(|join_handle| {
                join_handle.expect("task must have not failed to finish its execution!")
            })
            .boxed();

        Ok(PartiallyDelegated {
            sink_half: sink,
            delegated_stream: (spawned_boxed_task, notify),
        })
    }

    // if we want to send a message and don't care about response, we can don't need to reunite the split,
    // the sink itself is enough
    async fn send_without_response(&mut self, msg: Message) -> Result<(), GatewayClientError> {
        Ok(self.sink_half.send(msg).await?)
    }

    async fn merge(self) -> Result<WsConn, GatewayClientError> {
        let (stream_fut, notify) = self.delegated_stream;
        notify.notify();
        let stream = stream_fut.await?;
        // the error is thrown when trying to reunite sink and stream that did not originate
        // from the same split which is impossible to happen here
        Ok(self.sink_half.reunite(stream).unwrap())
    }
}

// we can either have the stream itself or an option to re-obtain it
// by notifying the future owning it to finish the execution and awaiting the result
// which should be almost immediate (or an invalid state which should never, ever happen)
enum SocketState<'a> {
    Available(WsConn),
    PartiallyDelegated(PartiallyDelegated<'a>),
    NotConnected,
    Invalid,
}

impl<'a> SocketState<'a> {
    fn is_available(&self) -> bool {
        match self {
            SocketState::Available(_) => true,
            _ => false,
        }
    }

    fn is_partially_delegated(&self) -> bool {
        match self {
            SocketState::PartiallyDelegated(_) => true,
            _ => false,
        }
    }

    fn is_established(&self) -> bool {
        match self {
            SocketState::Available(_) | SocketState::PartiallyDelegated(_) => true,
            _ => false,
        }
    }
}

pub struct GatewayClient<'a, R> {
    authenticated: bool,
    // can be String, string slices, `url::Url`, `http::Uri`, etc.
    gateway_address: R,
    gateway_identity: identity::PublicKey,
    local_identity: Arc<identity::KeyPair>,
    shared_key: Option<Arc<SharedKeys>>,
    connection: SocketState<'a>,
    packet_router: PacketRouter,
    response_timeout_duration: Duration,
}

impl<'a, R> GatewayClient<'static, R> {
    // TODO: put it all in a Config struct
    pub fn new(
        gateway_address: R,
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
        gateway_address: R,
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

    pub async fn close_connection(&mut self) -> Result<(), GatewayClientError> {
        if self.connection.is_partially_delegated() {
            self.recover_socket_connection().await?;
        }

        match std::mem::replace(&mut self.connection, SocketState::NotConnected) {
            SocketState::Available(mut socket) => Ok(socket.close(None).await?),
            SocketState::PartiallyDelegated(_) => {
                unreachable!("this branch should have never been reached!")
            }
            _ => Ok(()), // no need to do anything in those cases
        }
    }

    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError>
    where
        R: IntoClientRequest + Unpin + Clone,
    {
        let ws_stream = match connect_async(self.gateway_address.clone()).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(GatewayClientError::NetworkError(e)),
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

        let mut timeout = tokio::time::delay_for(self.response_timeout_duration);

        let mut res = None;
        while res.is_none() {
            tokio::select! {
                _ = &mut timeout => {
                    res = Some(Err(GatewayClientError::Timeout))
                }
                // just keep getting through socket buffer until we get to what we want...
                // (or we time out)
                msg = read_ws_stream_message(conn) => {
                    if let Err(err) = msg {
                        res = Some(Err(err));
                        break;
                    }
                    match msg.unwrap() {
                        Message::Binary(bin_msg) => {
                            self.packet_router.route_received(vec![bin_msg]);
                        }
                        Message::Text(txt_msg) => {
                            res = Some(ServerResponse::try_from(txt_msg).map_err(|_| GatewayClientError::MalformedResponse));
                        }
                        _ => (),
                    }
                }
            }
        }

        res.expect("response value should have been written in one of the branches!. If you see this error, please report a bug!")
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

        match &mut self.connection {
            SocketState::Available(ws_stream) => client_handshake(
                &mut DEFAULT_RNG,
                ws_stream,
                self.local_identity.as_ref(),
                self.gateway_identity,
            )
            .await
            .map_err(GatewayClientError::RegistrationFailure),
            _ => unreachable!(),
        }
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
            self.register().await?;
        }
        if self.authenticated {
            // if we are authenticated it means we MUST have an associated shared_key
            Ok(Arc::clone(&self.shared_key.as_ref().unwrap()))
        } else {
            Err(GatewayClientError::AuthenticationFailure)
        }
    }

    // TODO: possibly make responses optional
    pub async fn send_sphinx_packet(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        let msg = BinaryRequest::new_forward_request(address, packet).into_ws_message(
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
                    )?
                }
                _ => unreachable!(),
            };

        self.connection = SocketState::PartiallyDelegated(partially_delegated);
        Ok(())
    }

    pub async fn authenticate_and_start(&mut self) -> Result<Arc<SharedKeys>, GatewayClientError>
    where
        R: IntoClientRequest + Unpin + Clone,
    {
        if !self.connection.is_established() {
            self.establish_connection().await?;
        }
        let shared_key = self.perform_initial_authentication().await?;

        // that call is NON-blocking
        self.start_listening_for_mixnet_messages()?;

        Ok(shared_key)
    }
}
