// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::bandwidth::Bandwidth;
use crate::node::client_handling::websocket::connection_handler::{
    ClientDetails, FreshHandler, SocketStream,
};
use crate::node::client_handling::websocket::message_receiver::MixMessageReceiver;
use futures::{SinkExt, StreamExt};
use gateway_requests::iv::IV;
use gateway_requests::types::{BinaryRequest, ServerResponse};
use gateway_requests::{BinaryResponse, ClientControlRequest};
use log::*;
use rand::{CryptoRng, Rng};
use std::convert::TryFrom;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::{protocol::Message, Error as WsError};

pub(crate) struct AuthenticatedHandler<R, S> {
    inner: FreshHandler<R, S>,
    client: ClientDetails,
    mix_receiver: MixMessageReceiver,
}

impl<R, S> Drop for AuthenticatedHandler<R, S> {
    fn drop(&mut self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
    }
}

impl<R, S> AuthenticatedHandler<R, S>
where
    // TODO: those trait bounds here don't really make sense....
    R: Rng + CryptoRng,
{
    pub(crate) fn upgrade(
        fresh: FreshHandler<R, S>,
        client: ClientDetails,
        mix_receiver: MixMessageReceiver,
    ) -> Self {
        AuthenticatedHandler {
            inner: fresh,
            client,
            mix_receiver,
        }
    }

    fn disconnect(&self) {
        self.inner
            .active_clients_store
            .disconnect(self.client.address)
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
                if let Err(err) = self
                    .inner
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
            Err(e) => ServerResponse::new_error(format!("{:?}", e)),
        }
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
    pub(crate) async fn listen_for_requests(&mut self)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        trace!("Started listening for ALL incoming requests...");

        loop {
            tokio::select! {
                socket_msg = self.inner.read_websocket_message() => {
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
                        if let Err(err) = self.inner.send_websocket_message(response).await {
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
