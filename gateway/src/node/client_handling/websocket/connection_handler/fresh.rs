// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::clients_handler::{
    ClientsHandlerRequest, ClientsHandlerRequestSender, ClientsHandlerResponse,
};
use crate::node::client_handling::websocket::connection_handler::{
    AuthenticatedHandler, ClientDetails, InitialAuthResult, SocketStream,
};
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
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
use gateway_requests::types::{ClientControlRequest, ServerResponse};
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use nymsphinx::DestinationAddressBytes;
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

pub(crate) struct FreshHandler<R, S> {
    rng: R,
    local_identity: Arc<identity::KeyPair>,
    pub(crate) aggregated_verification_key: VerificationKey,
    pub(crate) clients_handler_sender: ClientsHandlerRequestSender,
    pub(crate) outbound_mix_sender: MixForwardingSender,
    pub(crate) socket_connection: SocketStream<S>,
    pub(crate) storage: GatewayStorage,
}

impl<R, S> FreshHandler<R, S>
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
        FreshHandler {
            rng,
            clients_handler_sender,
            outbound_mix_sender,
            socket_connection: SocketStream::RawTcp(conn),
            local_identity,
            aggregated_verification_key,
            storage,
        }
    }

    pub(crate) async fn perform_websocket_handshake(&mut self) -> Result<(), WsError>
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

    pub(crate) async fn read_websocket_message(&mut self) -> Option<Result<Message, WsError>>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    pub(crate) async fn send_websocket_message(&mut self, msg: Message) -> Result<(), WsError>
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
                // won't accept anything else (like bandwidth) without prior authentication
                _ => InitialAuthResult::new_error("invalid request - authentication is required"),
            }
        } else {
            InitialAuthResult::new_error("malformed request")
        }
    }

    /// Listens for only a subset of possible client requests, i.e. for those that can either
    /// result in client getting registered or authenticated. All other requests, such as forwarding
    /// sphinx packets considered an error and terminate the connection.
    pub(crate) async fn perform_initial_authentication(
        mut self,
    ) -> Option<AuthenticatedHandler<R, S>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        trace!("Started waiting for authenticate/register request...");

        while let Some(msg) = self.read_websocket_message().await {
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

                    if let Err(err) = self
                        .send_websocket_message(auth_result.server_response.into())
                        .await
                    {
                        debug!("Failed to send authentication response - {}", err);
                        return None;
                    }

                    return auth_result.client_details.map(|client_details| {
                        AuthenticatedHandler::upgrade(self, client_details, mix_receiver)
                    });
                }
                Message::Binary(_) => {
                    // perhaps logging level should be reduced here, let's leave it for now and see what happens
                    // if client is working correctly, this should have never happened
                    warn!("possibly received a sphinx packet without prior authentication. Request is going to be ignored");
                    if let Err(err) = self
                        .send_websocket_message(
                            ServerResponse::new_error(
                                "binary request without prior authentication",
                            )
                            .into(),
                        )
                        .await
                    {
                        debug!(
                            "Failed to send error response during authentication - {}",
                            err
                        )
                    }
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
        super::handle_connection(self).await
    }
}
