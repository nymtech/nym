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

use crate::node::client_handling::clients_handler::{
    ClientsHandlerRequest, ClientsHandlerRequestSender, ClientsHandlerResponse,
};
use crate::node::client_handling::websocket::message_receiver::{
    MixMessageReceiver, MixMessageSender,
};
use crate::node::mixnet_handling::sender::OutboundMixMessageSender;
use crypto::asymmetric::identity;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::authentication::iv::AuthenticationIV;
use gateway_requests::registration::handshake::error::HandshakeError;
use gateway_requests::registration::handshake::{gateway_handshake, SharedKey, DEFAULT_RNG};
use gateway_requests::types::{BinaryRequest, ClientControlRequest, ServerResponse};
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::{prelude::*, stream::StreamExt};
use tokio_tungstenite::{
    tungstenite::{protocol::Message, Error as WsError},
    WebSocketStream,
};

//// TODO: note for my future self to consider the following idea:
//// split the socket connection into sink and stream
//// stream will be for reading explicit requests
//// and sink for pumping responses AND mix traffic
//// but as byproduct this might (or might not) break the clean "SocketStream" enum here

enum SocketStream<S> {
    RawTCP(S),
    UpgradedWebSocket(WebSocketStream<S>),
    Invalid,
}

impl<S> SocketStream<S> {
    fn is_websocket(&self) -> bool {
        match self {
            SocketStream::UpgradedWebSocket(_) => true,
            _ => false,
        }
    }
}

pub(crate) struct Handle<S> {
    remote_address: Option<DestinationAddressBytes>,
    shared_key: Option<SharedKey>,
    clients_handler_sender: ClientsHandlerRequestSender,
    outbound_mix_sender: OutboundMixMessageSender,
    socket_connection: SocketStream<S>,

    local_identity: Arc<identity::KeyPair>,
}

impl<S> Handle<S> {
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(
        conn: S,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: OutboundMixMessageSender,
        local_identity: Arc<identity::KeyPair>,
    ) -> Self {
        Handle {
            remote_address: None,
            shared_key: None,
            clients_handler_sender,
            outbound_mix_sender,
            socket_connection: SocketStream::RawTCP(conn),
            local_identity,
        }
    }

    async fn perform_websocket_handshake(&mut self) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.socket_connection =
            match std::mem::replace(&mut self.socket_connection, SocketStream::Invalid) {
                SocketStream::RawTCP(conn) => {
                    // TODO: perhaps in the future, rather than panic here (and uncleanly shut tcp stream)
                    // return a result with an error?
                    let ws_stream = match tokio_tungstenite::accept_async(conn).await {
                        Ok(ws_stream) => ws_stream,
                        // note that socket will remain in `Invalid` state here, but that's
                        // absolutely fine because due to returned error the handler
                        // should terminate immediately
                        Err(err) => return Err(err),
                    };
                    SocketStream::UpgradedWebSocket(ws_stream)
                }
                other => other,
            };
        Ok(())
    }

    async fn perform_registration_handshake(
        &mut self,
        init_msg: Vec<u8>,
    ) -> Result<SharedKey, HandshakeError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        debug_assert!(self.socket_connection.is_websocket());
        match &mut self.socket_connection {
            SocketStream::UpgradedWebSocket(ws_stream) => {
                gateway_handshake(
                    &mut DEFAULT_RNG,
                    ws_stream,
                    self.local_identity.as_ref(),
                    init_msg,
                )
                .await
            }
            _ => unreachable!(),
        }
    }

    async fn next_websocket_request(&mut self) -> Option<Result<Message, WsError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn send_websocket_response(&mut self, msg: Message) -> Result<(), WsError>
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

    async fn send_websocket_sphinx_packets(&mut self, packets: Vec<Vec<u8>>) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let messages: Vec<Result<Message, WsError>> = packets
            .into_iter()
            .map(|packet| Ok(Message::Binary(packet)))
            .collect();
        let mut send_stream = futures::stream::iter(messages);
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => {
                ws_stream.send_all(&mut send_stream).await
            }
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    fn disconnect(&self) {
        // if we never established what is the address of the client, its connection was never
        // announced hence we do not need to send 'disconnect' message
        self.remote_address.as_ref().map(|addr| {
            self.clients_handler_sender
                .unbounded_send(ClientsHandlerRequest::Disconnect(addr.clone()))
                .unwrap();
        });
    }

    async fn handle_binary(&self, bin_msg: Vec<u8>) -> Message {
        trace!("Handling binary message (presumably sphinx packet)");

        match BinaryRequest::try_from_encrypted_bytes(
            bin_msg,
            self.shared_key
                .as_ref()
                .expect("no shared key present even though we authenticated the client!"),
        ) {
            Err(e) => ServerResponse::new_error(e.to_string()),
            Ok(request) => match request {
                // currently only a single type exists
                BinaryRequest::ForwardSphinx {
                    address,
                    sphinx_packet,
                } => {
                    // we know data has correct size (but nothing else besides of it)
                    self.outbound_mix_sender
                        .unbounded_send((address, sphinx_packet))
                        .unwrap();
                    ServerResponse::Send { status: true }
                }
            },
        }
        .into()
    }

    async fn handle_authenticate(
        &mut self,
        address: String,
        enc_address: String,
        iv: String,
        mix_sender: MixMessageSender,
    ) -> ServerResponse {
        let address = match DestinationAddressBytes::try_from_base58_string(address) {
            Ok(address) => address,
            Err(e) => {
                trace!("failed to parse received DestinationAddress: {:?}", e);
                return ServerResponse::new_error("malformed destination address").into();
            }
        };

        let encrypted_address = match EncryptedAddressBytes::try_from_base58_string(enc_address) {
            Ok(address) => address,
            Err(e) => {
                trace!("failed to parse received encrypted address: {:?}", e);
                return ServerResponse::new_error("malformed encrypted address").into();
            }
        };

        let iv = match AuthenticationIV::try_from_base58_string(iv) {
            Ok(iv) => iv,
            Err(e) => {
                trace!("failed to parse received IV {:?}", e);
                return ServerResponse::new_error("malformed iv").into();
            }
        };

        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request = ClientsHandlerRequest::Authenticate(
            address.clone(),
            encrypted_address,
            iv,
            mix_sender,
            res_sender,
        );
        self.clients_handler_sender
            .unbounded_send(clients_handler_request)
            .unwrap(); // the receiver MUST BE alive

        match res_receiver.await.unwrap() {
            ClientsHandlerResponse::Authenticate(shared_key) => {
                if shared_key.is_some() {
                    self.remote_address = Some(address);
                    self.shared_key = shared_key;
                    ServerResponse::Authenticate { status: true }
                } else {
                    ServerResponse::Authenticate { status: false }
                }
            }
            ClientsHandlerResponse::Error(e) => {
                error!("Authentication unexpectedly failed - {}", e);
                ServerResponse::Error {
                    message: "unexpected failure".into(),
                }
            }
            _ => panic!("received response to wrong query!"), // this should NEVER happen
        }
    }

    fn extract_remote_identity_from_register_init(init_data: &[u8]) -> Option<identity::PublicKey> {
        if init_data.len() < identity::PUBLIC_KEY_LENGTH {
            None
        } else {
            identity::PublicKey::from_bytes(&init_data[..identity::PUBLIC_KEY_LENGTH]).ok()
        }
    }

    async fn handle_register(
        &mut self,
        init_data: Vec<u8>,
        mix_sender: MixMessageSender,
    ) -> ServerResponse
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        // not entirely sure how to it more "nicely"...
        // hopefully, eventually this will go away once client's identity is known beforehand
        let remote_identity = match Self::extract_remote_identity_from_register_init(&init_data) {
            Some(address) => address,
            None => return ServerResponse::new_error("malformed request"),
        };
        let remote_address = remote_identity.derive_address();

        let derived_shared_key = match self.perform_registration_handshake(init_data).await {
            Ok(shared_key) => shared_key,
            Err(err) => {
                return ServerResponse::new_error(format!(
                    "failed to perform the handshake - {}",
                    err
                ))
            }
        };

        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request = ClientsHandlerRequest::Register(
            remote_address.clone(),
            derived_shared_key,
            mix_sender,
            res_sender,
        );

        self.clients_handler_sender
            .unbounded_send(clients_handler_request)
            .unwrap(); // the receiver MUST BE alive

        match res_receiver.await.unwrap() {
            // currently register can't fail (as in if all machines are working correctly and you
            // managed to complete registration handshake)
            ClientsHandlerResponse::Register(status) => {
                self.remote_address = Some(remote_address);
                ServerResponse::Register { status }
            }
            ClientsHandlerResponse::Error(e) => {
                error!("Post-handshake registration unexpectedly failed - {}", e);
                ServerResponse::Error {
                    message: "unexpected failure".into(),
                }
            }
            _ => panic!("received response to wrong query!"), // this should NEVER happen
        }
    }

    // currently there are no valid control messages you can send after authentication
    async fn handle_text(&mut self, _: String) -> Message {
        trace!("Handling text message (presumably control message)");

        error!("Currently there are no text messages besides 'Authenticate' and 'Register' and they were already dealt with!");
        ServerResponse::new_error("invalid request").into()
    }

    async fn handle_request(&mut self, raw_request: Message) -> Option<Message> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // desktop nym-client websocket as I've manually handled everything there
        match raw_request {
            Message::Binary(bin_msg) => Some(self.handle_binary(bin_msg).await),
            Message::Text(text_msg) => Some(self.handle_text(text_msg).await),
            _ => None,
        }
    }

    /// Handles data that resembles request to either start registration handshake or perform
    /// authentication.
    async fn handle_initial_authentication_request(
        &mut self,
        mix_sender: MixMessageSender,
        raw_request: String,
    ) -> ServerResponse
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Ok(request) = ClientControlRequest::try_from(raw_request) {
            match request {
                ClientControlRequest::Authenticate {
                    address,
                    enc_address,
                    iv,
                } => {
                    self.handle_authenticate(address, enc_address, iv, mix_sender)
                        .await
                }
                ClientControlRequest::RegisterHandshakeInitRequest { data } => {
                    self.handle_register(data, mix_sender).await
                }
            }
        } else {
            // TODO: is this a malformed request or rather a network error and
            // connection should be terminated?
            ServerResponse::new_error("malformed request")
        }
    }

    /// Listens for only a subset of possible client requests, i.e. for those that can either
    /// result in client getting registered or authenticated. All other requests, such as forwarding
    /// sphinx packets are ignored.
    async fn wait_for_initial_authentication(&mut self) -> Option<MixMessageReceiver>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        trace!("Started waiting for authenticate/register request...");

        while let Some(msg) = self.next_websocket_request().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(err) => {
                    error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                    break;
                }
            };

            if msg.is_close() {
                break;
            }

            let (mix_sender, mix_receiver) = mpsc::unbounded();

            // ONLY handle 'Authenticate' or 'Register' requests, ignore everything else
            let response = match msg {
                Message::Close(_) => break,
                Message::Text(text_msg) => {
                    self.handle_initial_authentication_request(mix_sender, text_msg)
                        .await
                }
                Message::Binary(_) => {
                    // perhaps logging level should be reduced here, let's leave it for now and see what happens
                    // if client is working correctly, this should have never happened
                    warn!("possibly received a sphinx packet without prior authentication. Request is going to be ignored");
                    ServerResponse::new_error("binary request without prior authentication")
                }

                _ => continue,
            };

            let is_done = response.implies_successful_authentication();

            if let Err(err) = self.send_websocket_response(response.into()).await {
                warn!(
                    "Failed to send message over websocket: {}. Assuming the connection is dead.",
                    err
                );
                break;
            }

            // it means we successfully managed to perform authentication and announce our
            // presence to ClientsHandler
            if is_done {
                return Some(mix_receiver);
            }
        }
        None
    }

    /// Simultaneously listens for incoming client requests, which realistically should only be
    /// binary requests to forward sphinx packets, and for sphinx packets received from the mix
    /// network that should be sent back to the client.
    async fn listen_for_requests(&mut self, mut mix_receiver: MixMessageReceiver)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        trace!("Started listening for ALL incoming requests...");

        loop {
            tokio::select! {
                socket_msg = self.next_websocket_request() => {
                    if socket_msg.is_none() {
                        break;
                    }
                    let socket_msg = match socket_msg.unwrap() {
                        Ok(socket_msg) => socket_msg,
                        Err(err) => {
                            error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_request(socket_msg).await {
                        if let Err(err) = self.send_websocket_response(response).await {
                            warn!(
                                "Failed to send message over websocket: {}. Assuming the connection is dead.",
                                err
                            );
                            break;
                        }
                    }
                },
                mix_messages = mix_receiver.next() => {
                    let mix_messages = mix_messages.expect("sender was unexpectedly closed! this shouldn't have ever happened!");
                    if let Err(e) = self.send_websocket_sphinx_packets(mix_messages).await {
                        warn!("failed to send sphinx packets back to the client - {:?}, assuming the connection is dead", e);
                        break;
                    }
                }
            }
        }

        self.disconnect();
        trace!("The stream was closed!");
    }

    pub(crate) async fn start_handling(&mut self)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        if let Err(e) = self.perform_websocket_handshake().await {
            warn!(
                "Failed to complete WebSocket handshake - {:?}. Stopping the handler",
                e
            );
            return;
        }
        trace!("Managed to perform websocket handshake!");
        let mix_receiver = self.wait_for_initial_authentication().await;
        trace!("Performed initial authentication");
        match mix_receiver {
            Some(receiver) => self.listen_for_requests(receiver).await,
            None => trace!("But connection was closed during the process"),
        }
        trace!("The handler is done!");
    }
}
