// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::bandwidth::Bandwidth;
use crate::node::client_handling::clients_handler::{
    ClientsHandlerRequest, ClientsHandlerRequestSender, ClientsHandlerResponse,
};
use crate::node::client_handling::websocket::message_receiver::{
    MixMessageReceiver, MixMessageSender,
};
use crate::node::storage::GatewayStorage;
use coconut_interface::VerificationKey;
use crypto::asymmetric::identity;
use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use gateway_requests::authentication::encrypted_address::EncryptedAddressBytes;
use gateway_requests::iv::IV;
use gateway_requests::registration::handshake::error::HandshakeError;
use gateway_requests::registration::handshake::{gateway_handshake, SharedKeys};
use gateway_requests::types::{BinaryRequest, ClientControlRequest, ServerResponse};
use gateway_requests::BinaryResponse;
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use nymsphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use sqlx::Error;
use std::convert::TryFrom;
use std::future::Future;
use std::mem;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
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
    RawTcp(S),
    UpgradedWebSocket(WebSocketStream<S>),
    Invalid,
}

impl<S> SocketStream<S> {
    fn is_websocket(&self) -> bool {
        matches!(self, SocketStream::UpgradedWebSocket(_))
    }
}

struct ClientDetails {
    address: DestinationAddressBytes,
    shared_keys: SharedKeys,
}

struct InitialAuthResult {
    client_details: Option<ClientDetails>,
    server_response: ServerResponse,
}

impl InitialAuthResult {
    fn new(client_details: Option<ClientDetails>, server_response: ServerResponse) -> Self {
        InitialAuthResult {
            client_details,
            server_response,
        }
    }

    fn new_error<S: Into<String>>(msg: S) -> Self {
        InitialAuthResult {
            client_details: None,
            server_response: ServerResponse::new_error(msg),
        }
    }
}

pub(crate) struct AuthenticatedHandle<R, S> {
    inner: Handle<R, S>,
    client: ClientDetails,
    mix_receiver: MixMessageReceiver,
}

impl<R, S> AuthenticatedHandle<R, S>
where
    // TODO: those trait bounds here don't really make sense....
    R: Rng + CryptoRng,
{
    fn disconnect(&self) {
        self.inner
            .clients_handler_sender
            .unbounded_send(ClientsHandlerRequest::Disconnect(self.client.address))
            .unwrap();
    }

    // Note that it encrypts each message and slaps a MAC on it
    async fn send_websocket_unwrapped_sphinx_packets(
        &mut self,
        packets: Vec<Vec<u8>>,
    ) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // note: into_ws_message encrypts the requests and adds a MAC on it. Perhaps it should
        // be more explicit in the naming?
        let messages: Vec<Result<Message, WsError>> = packets
            .into_iter()
            .map(|received_message| {
                Ok(BinaryResponse::new_pushed_mix_message(received_message)
                    .into_ws_message(&self.client.shared_keys))
            })
            .collect();
        let mut send_stream = futures::stream::iter(messages);
        match self.inner.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => {
                ws_stream.send_all(&mut send_stream).await
            }
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn handle_bandwidth(&mut self, enc_credential: Vec<u8>, iv: Vec<u8>) -> ServerResponse {
        let iv = match IV::try_from_bytes(&iv) {
            Ok(iv) => iv,
            Err(e) => {
                trace!("failed to parse received IV {:?}", e);
                return ServerResponse::new_error("malformed iv");
            }
        };
        let credential = match ClientControlRequest::try_from_enc_bandwidth_credential(
            enc_credential,
            &self.client.shared_keys,
            iv,
        ) {
            Ok(c) => c,
            Err(e) => {
                return ServerResponse::new_error(e.to_string());
            }
        };

        if !credential.verify(&self.inner.aggregated_verification_key) {
            return ServerResponse::Bandwidth { status: false };
        }

        match Bandwidth::try_from(credential) {
            Ok(bandwidth) => {
                let mut bandwidth_value = bandwidth.value();
                if bandwidth_value > i64::MAX as u64 {
                    // note that this would have represented more than 1 exabyte,
                    // which is like 125,000 worth of hard drives so I don't think we have
                    // to worry about it for now...
                    warn!("Somehow we received bandwidth value higher than 9223372036854775807. Going to cap it at that amount.");
                    bandwidth_value = i64::MAX as u64;
                }

                // the unwrap in remote address is fine as we have already ensured the address has been set
                if let Err(err) = self.inner
                    .storage
                    .increase_bandwidth(self.client.address, bandwidth_value as i64)
                    .await
                {
                    error!(
                        "We failed to increase the bandwidth of {}! - {}",
                        self.client.address.as_base58_string(),
                        err
                    );
                    ServerResponse::new_error("Internal gateway storage error")
                } else {
                    ServerResponse::Bandwidth { status: true }
                }
            }
            Err(e) => ServerResponse::Error {
                message: format!("{:?}", e),
            },
        }.into()
    }

    async fn handle_binary(&self, bin_msg: Vec<u8>) -> Message {
        trace!("Handling binary message (presumably sphinx packet)");

        // if no available bandwidth, exit straightaway so we wouldn't need to waste CPU trying to unwrap
        // the received packet

        // this function decrypts the request and checks the MAC
        match BinaryRequest::try_from_encrypted_tagged_bytes(bin_msg, &self.client.shared_keys) {
            Err(e) => ServerResponse::new_error(e.to_string()),
            Ok(request) => match request {
                // currently only a single type exists
                BinaryRequest::ForwardSphinx(mix_packet) => {
                    // for now let's just use actual size of the sphinx packet. there's a tiny bit of overhead
                    // we're not including (but it's literally like 2 bytes) when the packet is framed
                    let consumed_bandwidth = mix_packet.sphinx_packet().len() as i64;

                    let available_bandwidth = match self
                        .inner
                        .storage
                        .get_available_bandwidth(self.client.address)
                        .await
                    {
                        Err(err) => {
                            error!(
                                "We failed perform bandwidth lookup of {}! - {}",
                                self.client.address.as_base58_string(),
                                err
                            );
                            return ServerResponse::new_error("Internal gateway storage error")
                                .into();
                        }
                        Ok(None) => {
                            return ServerResponse::new_error("No bandwidth available").into()
                        }
                        Ok(Some(available_bandwidth)) => available_bandwidth,
                    };

                    if available_bandwidth < consumed_bandwidth {
                        return ServerResponse::new_error("Insufficient bandwidth available")
                            .into();
                    }

                    if let Err(err) = self
                        .inner
                        .storage
                        .consume_bandwidth(self.client.address, consumed_bandwidth)
                        .await
                    {
                        error!(
                            "We failed to consume the bandwidth of {}! - {}",
                            self.client.address.as_base58_string(),
                            err
                        );
                        ServerResponse::new_error("Internal gateway storage error")
                    } else {
                        self.inner
                            .outbound_mix_sender
                            .unbounded_send(mix_packet)
                            .unwrap();
                        ServerResponse::Send { status: true }
                    }
                }
            },
        }
        .into()
    }

    // currently the bandwidth credential request is the only one we can receive after
    // authentication
    async fn handle_text(&mut self, raw_request: String) -> Message {
        if let Ok(request) = ClientControlRequest::try_from(raw_request) {
            match request {
                ClientControlRequest::BandwidthCredential { enc_credential, iv } => {
                    self.handle_bandwidth(enc_credential, iv).await.into()
                }
                _ => ServerResponse::new_error("invalid request").into(),
            }
        } else {
            ServerResponse::new_error("malformed request").into()
        }
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

    /// Simultaneously listens for incoming client requests, which realistically should only be
    /// binary requests to forward sphinx packets, and for sphinx packets received from the mix
    /// network that should be sent back to the client.
    async fn listen_for_requests(&mut self)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        trace!("Started listening for ALL incoming requests...");

        loop {
            tokio::select! {
                socket_msg = self.inner.next_websocket_request() => {
                    let socket_msg = match socket_msg {
                        None => break,
                        Some(Ok(socket_msg)) => socket_msg,
                        Some(Err(err)) => {
                            error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_request(socket_msg).await {
                        if let Err(err) = self.inner.send_websocket_response(response).await {
                            warn!(
                                "Failed to send message over websocket: {}. Assuming the connection is dead.",
                                err
                            );
                            break;
                        }
                    }
                },
                mix_messages = self.mix_receiver.next() => {
                    let mix_messages = mix_messages.expect("sender was unexpectedly closed! this shouldn't have ever happened!");
                    if let Err(e) = self.send_websocket_unwrapped_sphinx_packets(mix_messages).await {
                        warn!("failed to send the unwrapped sphinx packets back to the client - {:?}, assuming the connection is dead", e);
                        break;
                    }
                }
            }
        }

        self.disconnect();
        trace!("The stream was closed!");
    }
}

pub(crate) struct Handle<R, S> {
    rng: R,
    clients_handler_sender: ClientsHandlerRequestSender,
    outbound_mix_sender: MixForwardingSender,
    socket_connection: SocketStream<S>,
    local_identity: Arc<identity::KeyPair>,
    aggregated_verification_key: VerificationKey,
    storage: GatewayStorage,
}

impl<R, S> Handle<R, S>
where
    R: Rng + CryptoRng,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(
        rng: R,
        conn: S,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: MixForwardingSender,
        local_identity: Arc<identity::KeyPair>,
        aggregated_verification_key: VerificationKey,
        storage: GatewayStorage,
    ) -> Self {
        Handle {
            rng,
            clients_handler_sender,
            outbound_mix_sender,
            socket_connection: SocketStream::RawTcp(conn),
            local_identity,
            aggregated_verification_key,
            storage,
        }
    }

    async fn perform_websocket_handshake(&mut self) -> Result<(), WsError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        self.socket_connection =
            match std::mem::replace(&mut self.socket_connection, SocketStream::Invalid) {
                SocketStream::RawTcp(conn) => {
                    // TODO: perhaps in the future, rather than panic here (and uncleanly shut tcp stream)
                    // return a result with an error?
                    let ws_stream = tokio_tungstenite::accept_async(conn).await?;
                    SocketStream::UpgradedWebSocket(ws_stream)
                }
                other => other,
            };
        Ok(())
    }

    async fn perform_registration_handshake(
        &mut self,
        init_msg: Vec<u8>,
    ) -> Result<SharedKeys, HandshakeError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        debug_assert!(self.socket_connection.is_websocket());
        match &mut self.socket_connection {
            SocketStream::UpgradedWebSocket(ws_stream) => {
                gateway_handshake(
                    &mut self.rng,
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

    async fn handle_authenticate(
        &mut self,
        address: String,
        enc_address: String,
        iv: String,
        mix_sender: MixMessageSender,
    ) -> InitialAuthResult {
        let address = match DestinationAddressBytes::try_from_base58_string(address) {
            Ok(address) => address,
            Err(e) => {
                trace!("failed to parse received DestinationAddress: {:?}", e);
                return InitialAuthResult::new_error("malformed destination address");
            }
        };

        let encrypted_address = match EncryptedAddressBytes::try_from_base58_string(enc_address) {
            Ok(address) => address,
            Err(e) => {
                trace!("failed to parse received encrypted address: {:?}", e);
                return InitialAuthResult::new_error("malformed encrypted address");
            }
        };

        let iv = match IV::try_from_base58_string(iv) {
            Ok(iv) => iv,
            Err(e) => {
                trace!("failed to parse received IV {:?}", e);
                return InitialAuthResult::new_error("malformed iv");
            }
        };

        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request = ClientsHandlerRequest::Authenticate(
            address,
            encrypted_address,
            iv,
            mix_sender,
            res_sender,
        );
        self.clients_handler_sender
            .unbounded_send(clients_handler_request)
            .unwrap(); // the receiver MUST BE alive

        match res_receiver.await.unwrap() {
            ClientsHandlerResponse::Authenticate(shared_keys) => {
                let status = shared_keys.is_some();
                let client_details = shared_keys.map(|shared_keys| ClientDetails {
                    address,
                    shared_keys,
                });
                InitialAuthResult::new(client_details, ServerResponse::Authenticate { status })
            }
            ClientsHandlerResponse::Error(e) => {
                error!("Authentication unexpectedly failed - {}", e);
                InitialAuthResult::new_error(format!("Authentication failure - {}", e))
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
    ) -> InitialAuthResult
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        // not entirely sure how to it more "nicely"...
        // hopefully, eventually this will go away once client's identity is known beforehand
        let remote_identity = match Self::extract_remote_identity_from_register_init(&init_data) {
            Some(address) => address,
            None => return InitialAuthResult::new_error("malformed request"),
        };
        let remote_address = remote_identity.derive_destination_address();

        let derived_shared_key = match self.perform_registration_handshake(init_data).await {
            Ok(shared_key) => shared_key,
            Err(err) => {
                return InitialAuthResult::new_error(format!(
                    "failed to perform the handshake - {}",
                    err
                ))
            }
        };

        // TODO: this will go away in few commits
        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request = ClientsHandlerRequest::Register(
            remote_address,
            derived_shared_key.clone(),
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
                let mut client_details = None;
                if status {
                    client_details = Some(ClientDetails {
                        address: remote_address,
                        shared_keys: derived_shared_key,
                    });
                }
                InitialAuthResult::new(client_details, ServerResponse::Register { status })
            }
            ClientsHandlerResponse::Error(e) => {
                error!("Post-handshake registration unexpectedly failed - {}", e);
                InitialAuthResult::new_error(format!("Registration failure - {}", e))
            }
            _ => panic!("received response to wrong query!"), // this should NEVER happen
        }
    }

    async fn handle_bandwidth(&mut self, enc_credential: Vec<u8>, iv: Vec<u8>) -> ServerResponse {
        todo!("disallow")
    }

    async fn handle_binary(&mut self, bin_msg: Vec<u8>) -> Message {
        todo!()
    }

    /// Handles data that resembles request to either start registration handshake or perform
    /// authentication.
    async fn handle_initial_authentication_request(
        &mut self,
        mix_sender: MixMessageSender,
        raw_request: String,
    ) -> InitialAuthResult
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
                _ => InitialAuthResult::new_error("invalid request"),
            }
        } else {
            // TODO: is this a malformed request or rather a network error and
            // connection should be terminated?
            InitialAuthResult::new_error("malformed request")
        }
    }

    /// Listens for only a subset of possible client requests, i.e. for those that can either
    /// result in client getting registered or authenticated. All other requests, such as forwarding
    /// sphinx packets considered an error and terminate the connection.
    async fn perform_initial_authentication(mut self) -> Option<AuthenticatedHandle<R, S>>
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

            // ONLY handle 'Authenticate' or 'Register' requests, ignore everything else
            match msg {
                Message::Close(_) => break,
                Message::Text(text_msg) => {
                    let (mix_sender, mix_receiver) = mpsc::unbounded();
                    let auth_result = self
                        .handle_initial_authentication_request(mix_sender, text_msg)
                        .await;
                    todo!()
                }
                Message::Binary(_) => {
                    // perhaps logging level should be reduced here, let's leave it for now and see what happens
                    // if client is working correctly, this should have never happened
                    warn!("possibly received a sphinx packet without prior authentication. Request is going to be ignored");
                    self.send_websocket_response(
                        ServerResponse::new_error("binary request without prior authentication")
                            .into(),
                    )
                    .await;
                    return None;
                }

                _ => continue,
            };
        }
        None
    }

    pub(crate) async fn start_handling(self)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        handle_connection(self).await
    }
}

pub(crate) async fn handle_connection<R, S>(mut handle: Handle<R, S>)
where
    R: Rng + CryptoRng,
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if let Err(err) = handle.perform_websocket_handshake().await {
        warn!(
            "Failed to complete WebSocket handshake - {}. Stopping the handler",
            err
        );
        return;
    }

    trace!("Managed to perform websocket handshake!");

    if let Some(mut auth_handle) = handle.perform_initial_authentication().await {
        auth_handle.listen_for_requests().await
    } else {
        warn!("Authentication has failed")
    }
    trace!("The handler is done!");
}
