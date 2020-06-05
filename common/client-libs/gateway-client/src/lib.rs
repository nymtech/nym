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

use futures::stream::{SplitSink, SplitStream};
use futures::{channel::mpsc, future::BoxFuture, FutureExt, SinkExt, StreamExt};
use gateway_requests::auth_token::{AuthToken, AuthTokenConversionError};
use gateway_requests::{BinaryRequest, ClientControlRequest, ServerResponse};
use log::*;
use nymsphinx::{DestinationAddressBytes, SphinxPacket};
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

// TODO: some batching mechanism to allow reading and sending more than a single packet through
pub type MixnetMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type MixnetMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

pub type AcknowledgementSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

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
    AuthenticationFailure,
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

// better human readable representation of the error, mostly so that GatewayClientError
// would implement std::error::Error
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
            GatewayClientError::AuthenticationFailure => write!(f, "authentication failure"),
            GatewayClientError::GatewayError(err) => {
                write!(f, "gateway returned an error response - {}", err)
            }
        }
    }
}

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
        mixnet_message_sender: MixnetMessageSender,
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
                    msg = stream.next() => {
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
                        // TODO: match to determine if it's ack or message
                            Message::Binary(bin_msg) => {
                                // TODO: some batching mechanism to allow reading and sending more than
                                // one packet at the time, because the receiver can easily handle it
                                mixnet_message_sender.unbounded_send(vec![bin_msg]).unwrap()
                            },
                            // I think that in the future we should perhaps have some sequence number system, i.e.
                            // so each request/reponse pair can be easily identified, so that if messages are
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

pub struct GatewayClient<'a, R: IntoClientRequest + Unpin + Clone> {
    authenticated: bool,
    // can be String, string slices, `url::Url`, `http::Uri`, etc.
    gateway_address: R,
    our_address: DestinationAddressBytes,
    auth_token: Option<AuthToken>,
    connection: SocketState<'a>,
    mixnet_message_sender: MixnetMessageSender,
    ack_sender: AcknowledgementSender,
    response_timeout_duration: Duration,
}

impl<'a, R: IntoClientRequest + Unpin + Clone> Drop for GatewayClient<'a, R> {
    fn drop(&mut self) {
        // TODO to fix forcibly closing connection (although now that I think about it,
        // I'm not sure this would do it, as to fix the said issue we'd need graceful shutdowns)
    }
}

impl<'a, R> GatewayClient<'static, R>
where
    R: IntoClientRequest + Unpin + Clone,
{
    pub fn new(
        gateway_address: R,
        our_address: DestinationAddressBytes,
        auth_token: Option<AuthToken>,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        response_timeout_duration: Duration,
    ) -> Self {
        GatewayClient {
            authenticated: false,
            gateway_address,
            our_address,
            auth_token,
            connection: SocketState::NotConnected,
            mixnet_message_sender,
            ack_sender,
            response_timeout_duration,
        }
    }
    // TODO: extra constructor for JUST init registration so that it would look something like:
    pub fn new_init(
        gateway_address: R,
        our_address: DestinationAddressBytes,
        response_timeout_duration: Duration,
    ) -> Self {
        todo!()
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
                            self.mixnet_message_sender.unbounded_send(vec![bin_msg]).unwrap()
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
        let mut should_restart_mixnet_listener = false;
        if self.connection.is_partially_delegated() {
            self.recover_socket_connection().await?;
            should_restart_mixnet_listener = true;
        }

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
        self.start_listening_for_mixnet_messages()?;
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
        self.start_listening_for_mixnet_messages()?;
        Ok(authenticated)
    }

    // just a helper method to either call register or authenticate based on self.auth_token value
    pub async fn perform_initial_authentication(
        &mut self,
    ) -> Result<AuthToken, GatewayClientError> {
        if self.auth_token.is_some() {
            self.authenticate(None).await?;
        } else {
            self.register().await?;
        }
        if self.authenticated {
            // if we are authenticated it means we MUST have an associated auth_token
            Ok(self.auth_token.clone().unwrap())
        } else {
            Err(GatewayClientError::AuthenticationFailure)
        }
    }

    // TODO: possibly make responses optional
    pub async fn send_sphinx_packet(
        &mut self,
        address: SocketAddr,
        packet: SphinxPacket,
    ) -> Result<(), GatewayClientError> {
        if !self.authenticated {
            return Err(GatewayClientError::NotAuthenticated);
        }
        if !self.connection.is_established() {
            return Err(GatewayClientError::ConnectionNotEstablished);
        }
        let msg = BinaryRequest::new_forward_request(address, packet).into();
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

    fn start_listening_for_mixnet_messages(&mut self) -> Result<(), GatewayClientError> {
        unimplemented!("need extra logic to use 'ack_sender'");

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
                        self.mixnet_message_sender.clone(),
                    )?
                }
                _ => unreachable!(),
            };

        self.connection = SocketState::PartiallyDelegated(partially_delegated);
        Ok(())
    }
}
