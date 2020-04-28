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

use futures::{channel::mpsc, future::LocalBoxFuture, FutureExt, SinkExt, StreamExt};
use gateway_requests::auth_token::{AuthToken, AuthTokenConversionError};
use gateway_requests::{BinaryRequest, ClientControlRequest, ServerResponse};
use nymsphinx::DestinationAddressBytes;
use std::convert::TryFrom;
use std::fmt::{self, Error, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message, Error as WsError},
    WebSocketStream,
};

// TODO: combine the duplicate reading procedure, i.e.
/*
msg read from conn.next().await:
    if msg.is_none() {
        res = Some(Err(GatewayClientError::ConnectionAbruptlyClosed));
        break;
    }
    let msg = match msg.unwrap() {
        Ok(msg) => msg,
        Err(err) => {
            res = Some(Err(GatewayClientError::NetworkError(err)));
            break;
        }
    };
    match msg {
        // specific handling
    }
*/

// to be moved to different crate perhaps? We'll see
type SphinxPacketSender = mpsc::UnboundedSender<Vec<u8>>;

// type alias for not having to type the whole thing every single time
type WsConn = WebSocketStream<TcpStream>;

#[derive(Debug)]
pub enum GatewayClientError {
    ConnectionNotEstablished,
    GatewayError(String),
    NetworkError(WsError),
    NoAuthTokenAvailable,
    ConnectionAbruptlyClosed,
    MalformedResponse,
    NotAuthenticated,
    ConnectionInInvalidState,
    Timeout,
}

impl From<WsError> for GatewayClientError {
    fn from(err: WsError) -> Self {
        GatewayClientError::NetworkError(err)
    }
}

impl From<AuthTokenConversionError> for GatewayClientError {
    fn from(_err: AuthTokenConversionError) -> Self {
        GatewayClientError::MalformedResponse
    }
}

impl fmt::Display for GatewayClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            GatewayClientError::ConnectionNotEstablished => {
                write!(f, "connection to the gateway is not established")
            }
            GatewayClientError::NoAuthTokenAvailable => {
                write!(f, "no AuthToken was provided or obtained")
            }
            GatewayClientError::NotAuthenticated => write!(f, "client is not authenticated"),
            GatewayClientError::NetworkError(err) => {
                write!(f, "there was a network error - {}", err)
            }
            GatewayClientError::ConnectionAbruptlyClosed => {
                write!(f, "connection was abruptly closed")
            }
            GatewayClientError::Timeout => write!(f, "timed out"),
            GatewayClientError::MalformedResponse => write!(f, "received response was malformed"),
            GatewayClientError::ConnectionInInvalidState => write!(
                f,
                "connection is in an invalid state - please send a bug report"
            ),
            GatewayClientError::GatewayError(err) => {
                write!(f, "gateway returned an error response - {}", err)
            }
        }
    }
}

// we can either have the stream itself or an option to re-obtain it
// by notifying the future owning it to finish the execution and awaiting the result
// which should be almost immediate (or an invalid state which should never, ever happen)
enum SocketState<'a> {
    Available(WsConn),
    Delegated(
        LocalBoxFuture<'a, Result<WsConn, GatewayClientError>>,
        Arc<Notify>,
    ),
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

    fn is_delegated(&self) -> bool {
        match self {
            SocketState::Delegated(_, _) => true,
            _ => false,
        }
    }

    fn is_established(&self) -> bool {
        match self {
            SocketState::Available(_) | SocketState::Delegated(_, _) => true,
            _ => false,
        }
    }
}

pub struct GatewayClient<'a, R: IntoClientRequest + Unpin + Clone> {
    authenticated: bool,
    // can be String, string slices, `url::Url`, `http::Uri`, etc.
    gateway_address: R,
    our_address: DestinationAddressBytes,
    auth_token: Option<AuthToken>,
    connection: SocketState<'a>,
    sphinx_packet_sender: SphinxPacketSender,
    response_timeout_duration: Duration,
}

impl<'a, R> GatewayClient<'static, R>
where
    R: IntoClientRequest + Unpin + Clone,
{
    pub fn new(
        gateway_address: R,
        our_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
        sphinx_packet_sender: SphinxPacketSender,
        response_timeout_duration: Duration,
    ) -> Self {
        GatewayClient {
            authenticated: false,
            gateway_address,
            our_address,
            auth_token,
            connection: SocketState::NotConnected,
            sphinx_packet_sender,
            response_timeout_duration,
        }
    }

    pub async fn establish_connection(&mut self) -> Result<(), GatewayClientError> {
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
                msg = conn.next() => {
                    if msg.is_none() {
                        res = Some(Err(GatewayClientError::ConnectionAbruptlyClosed));
                        break;
                    }
                    let msg = match msg.unwrap() {
                        Ok(msg) => msg,
                        Err(err) => {
                            res = Some(Err(GatewayClientError::NetworkError(err)));
                            break;
                        }
                    };
                    match msg {
                        Message::Binary(bin_msg) => {
                            self.sphinx_packet_sender.unbounded_send(bin_msg).unwrap()
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

    // If we want to send a message, we need to have a full control over the socket,
    // as we need to be able to write the request and read the subsequent response
    async fn send_websocket_message(
        &mut self,
        msg: Message,
    ) -> Result<ServerResponse, GatewayClientError> {
        let mut should_restart_sphinx_listener = false;
        if self.connection.is_delegated() {
            self.recover_socket_connection().await?;
            should_restart_sphinx_listener = true;
        }

        let conn = match self.connection {
            SocketState::Available(ref mut conn) => conn,
            SocketState::Delegated(_, _) => {
                return Err(GatewayClientError::ConnectionInInvalidState)
            }
            SocketState::Invalid => return Err(GatewayClientError::ConnectionInInvalidState),
            SocketState::NotConnected => return Err(GatewayClientError::ConnectionNotEstablished),
        };
        conn.send(msg).await?;
        let response = self.read_control_response().await;

        if should_restart_sphinx_listener {
            self.start_listening_for_sphinx_packets()?;
        }
        response
    }

    pub async fn register(&mut self) -> Result<AuthToken, GatewayClientError> {
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        let msg = ClientControlRequest::new_register(self.our_address.clone()).into();
        let token = match self.send_websocket_message(msg).await? {
            ServerResponse::Register { token } => {
                self.authenticated = true;
                Ok(AuthToken::try_from_base58_string(token)?)
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => unreachable!(),
        }?;
        self.start_listening_for_sphinx_packets()?;
        Ok(token)
    }

    pub async fn authenticate(
        &mut self,
        auth_token: Option<AuthToken>,
    ) -> Result<bool, GatewayClientError> {
        if auth_token.is_none() && self.auth_token.is_none() {
            return Err(GatewayClientError::NoAuthTokenAvailable);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        // because of the previous check one of the unwraps MUST succeed
        let auth_token = auth_token.unwrap_or_else(|| self.auth_token.unwrap());

        let msg =
            ClientControlRequest::new_authenticate(self.our_address.clone(), auth_token).into();
        let authenticated = match self.send_websocket_message(msg).await? {
            ServerResponse::Authenticate { status } => {
                self.authenticated = status;
                Ok(status)
            }
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => unreachable!(),
        }?;
        self.start_listening_for_sphinx_packets()?;
        Ok(authenticated)
    }

    pub async fn send_sphinx_packet(
        &mut self,
        address: SocketAddr,
        packet: Vec<u8>,
    ) -> Result<bool, GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        let msg = BinaryRequest::new_forward_request(address, packet).into();
        match self.send_websocket_message(msg).await? {
            ServerResponse::Send { status } => Ok(status),
            ServerResponse::Error { message } => Err(GatewayClientError::GatewayError(message)),
            _ => unreachable!(),
        }
    }

    async fn recover_socket_connection(&mut self) -> Result<(), GatewayClientError> {
        if self.connection.is_available() {
            return Ok(());
        }
        if !self.connection.is_delegated() {
            return Err(GatewayClientError::ConnectionInInvalidState);
        }

        let (conn_fut, notify) = match std::mem::replace(&mut self.connection, SocketState::Invalid)
        {
            SocketState::Delegated(conn_fut, notify) => (conn_fut, notify),
            _ => unreachable!(),
        };

        // tell the future to wrap up whatever it's doing now
        notify.notify();
        self.connection = SocketState::Available(conn_fut.await?);
        Ok(())
    }

    // TODO: this can be potentially bad as we have no direct restrictions of ensuring it's called
    // within tokio runtime. Perhaps we should use the "old" way of passing explicit
    // runtime handle to the constructor and using that instead?
    fn start_listening_for_sphinx_packets(&mut self) -> Result<(), GatewayClientError> {
        if !self.connection.is_available() {
            return Err(GatewayClientError::ConnectionInInvalidState);
        }

        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        let notify = Arc::new(Notify::new());
        let notify_clone = Arc::clone(&notify);

        let mut extracted_connection =
            match std::mem::replace(&mut self.connection, SocketState::Invalid) {
                SocketState::Available(conn) => conn,
                _ => unreachable!(), // impossible due to initial check
            };

        let sphinx_packet_sender = self.sphinx_packet_sender.clone();
        let sphinx_receiver_future = async move {
            let mut should_return = false;
            while !should_return {
                tokio::select! {
                    _ = notify_clone.notified() => {
                        should_return = true;
                    }
                    msg = extracted_connection.next() => {
                        if msg.is_none() {
                            return Err(GatewayClientError::ConnectionAbruptlyClosed);
                        }
                        let msg = match msg.unwrap() {
                            Ok(msg) => msg,
                            Err(err) => {
                                return Err(GatewayClientError::NetworkError(err));
                            }
                        };
                        match msg {
                            Message::Binary(bin_msg) => {
                                sphinx_packet_sender.unbounded_send(bin_msg).unwrap()
                            }
                            _ => (),
                        };
                    }
                };
            }
            Ok(extracted_connection)
        };

        let spawned_boxed_task = tokio::spawn(sphinx_receiver_future)
            .map(|join_handle| join_handle.expect("task must have not failed to finish execution!"))
            .boxed();

        self.connection = SocketState::Delegated(spawned_boxed_task, notify);
        Ok(())
    }
}
