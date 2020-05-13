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

use super::types::{BinaryClientRequest, ClientRequest, ServerResponse};
use crate::client::{
    received_buffer::{
        ReceivedBufferMessage, ReceivedBufferRequestSender, ReconstructedMessagesReceiver,
    },
    topology_control::TopologyAccessor,
    InputMessage, InputMessageSender,
};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::chunking::split_and_prepare_payloads;
use nymsphinx::DestinationAddressBytes;
use std::convert::TryFrom;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{protocol::Message, Error as WsError},
    WebSocketStream,
};
use topology::NymTopology;

enum ReceivedResponseType {
    Binary,
    Text,
}

impl Default for ReceivedResponseType {
    fn default() -> Self {
        ReceivedResponseType::Binary
    }
}

pub(crate) struct Handler<T: NymTopology> {
    msg_input: InputMessageSender,
    buffer_requester: ReceivedBufferRequestSender,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
    socket: Option<WebSocketStream<TcpStream>>,
    received_response_type: ReceivedResponseType,
}

// clone is used to use handler on a new connection, which initially is `None`
impl<T: NymTopology> Clone for Handler<T> {
    fn clone(&self) -> Self {
        Handler {
            msg_input: self.msg_input.clone(),
            buffer_requester: self.buffer_requester.clone(),
            self_address: self.self_address.clone(),
            topology_accessor: self.topology_accessor.clone(),
            socket: None,
            received_response_type: Default::default(),
        }
    }
}

impl<T: NymTopology> Drop for Handler<T> {
    fn drop(&mut self) {
        self.buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
            .expect("the buffer request failed!")
    }
}

impl<T: NymTopology> Handler<T> {
    pub(crate) fn new(
        msg_input: InputMessageSender,
        buffer_requester: ReceivedBufferRequestSender,
        self_address: DestinationAddressBytes,
        topology_accessor: TopologyAccessor<T>,
    ) -> Self {
        Handler {
            msg_input,
            buffer_requester,
            self_address,
            topology_accessor,
            socket: None,
            received_response_type: Default::default(),
        }
    }

    fn handle_text_send(&mut self, msg: String, recipient_address: String) -> ServerResponse {
        let message_bytes = msg.into_bytes();

        let address = match DestinationAddressBytes::try_from_base58_string(recipient_address) {
            Ok(address) => address,
            Err(e) => {
                trace!("failed to parse received DestinationAddress: {:?}", e);
                return ServerResponse::new_error("malformed destination address");
            }
        };

        // in case the message is too long and needs to be split into multiple packets:
        let split_message = split_and_prepare_payloads(&message_bytes);
        for message_fragment in split_message {
            let input_msg = InputMessage::new(address.clone(), message_fragment);
            self.msg_input.unbounded_send(input_msg).unwrap();
        }

        self.received_response_type = ReceivedResponseType::Text;

        ServerResponse::Send
    }

    async fn handle_text_get_clients(&mut self) -> ServerResponse {
        match self.topology_accessor.get_all_clients().await {
            Some(clients) => {
                let client_keys = clients.into_iter().map(|client| client.pub_key).collect();
                ServerResponse::GetClients {
                    clients: client_keys,
                }
            }
            None => ServerResponse::new_error("invalid network topology"),
        }
    }

    fn handle_text_self_address(&self) -> ServerResponse {
        ServerResponse::SelfAddress {
            address: self.self_address.to_base58_string(),
        }
    }

    async fn handle_text_message(&mut self, msg: String) -> Message {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg.clone());

        match ClientRequest::try_from(msg) {
            Err(e) => ServerResponse::Error {
                message: format!("received invalid request. err: {:?}", e),
            }
            .into(),
            Ok(req) => match req {
                ClientRequest::Send { message, recipient } => {
                    self.handle_text_send(message, recipient)
                }
                ClientRequest::GetClients => self.handle_text_get_clients().await,
                ClientRequest::SelfAddress => self.handle_text_self_address(),
            }
            .into(),
        }
    }

    async fn handle_binary_send(
        &mut self,
        address: DestinationAddressBytes,
        data: Vec<u8>,
    ) -> ServerResponse {
        // in case the message is too long and needs to be split into multiple packets:
        let split_message = split_and_prepare_payloads(&data);
        for message_fragment in split_message {
            let input_msg = InputMessage::new(address.clone(), message_fragment);
            self.msg_input.unbounded_send(input_msg).unwrap();
        }

        self.received_response_type = ReceivedResponseType::Binary;
        ServerResponse::Send
    }

    // if it's binary we assume it's a sphinx packet formatted the same way as we'd have sent
    // it to the gateway
    async fn handle_binary_message(&mut self, msg: Vec<u8>) -> Message {
        debug!("Handling binary message request");

        self.received_response_type = ReceivedResponseType::Binary;
        // make sure it is correctly formatted
        let binary_request = BinaryClientRequest::try_from_bytes(&msg);
        if binary_request.is_none() {
            return ServerResponse::new_error("invalid binary request").into();
        }
        match binary_request.unwrap() {
            BinaryClientRequest::Send {
                recipient_address,
                data,
            } => self.handle_binary_send(recipient_address, data).await,
        }
        .into()
    }

    async fn handle_request(&mut self, raw_request: Message) -> Option<Message> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // old version of this file.
        match raw_request {
            Message::Text(text_message) => Some(self.handle_text_message(text_message).await),
            Message::Binary(binary_message) => {
                Some(self.handle_binary_message(binary_message).await)
            }
            _ => None,
        }
    }

    async fn push_websocket_received_plaintexts(
        &mut self,
        messages_bytes: Vec<Vec<u8>>,
    ) -> Result<(), WsError> {
        let response_messages: Vec<_> = match self.received_response_type {
            ReceivedResponseType::Binary => messages_bytes
                .into_iter()
                .map(|msg| Ok(Message::Binary(msg)))
                .collect(),
            ReceivedResponseType::Text => {
                let mut decoded_messages = Vec::new();
                // either all succeed or all fall back
                let mut did_fail = false;
                for message in messages_bytes.iter() {
                    match std::str::from_utf8(message) {
                        Ok(msg) => decoded_messages.push(msg),
                        Err(err) => {
                            did_fail = true;
                            warn!("Invalid UTF-8 sequence in response message - {:?}", err);
                            break;
                        }
                    }
                }
                if did_fail {
                    messages_bytes
                        .into_iter()
                        .map(|msg| Ok(Message::Binary(msg)))
                        .collect()
                } else {
                    decoded_messages
                        .into_iter()
                        .map(|msg| Ok(Message::Text(msg.to_string())))
                        .collect()
                }
            }
        };

        let mut send_stream = futures::stream::iter(response_messages);
        self.socket
            .as_mut()
            .unwrap()
            .send_all(&mut send_stream)
            .await
    }

    async fn send_websocket_response(&mut self, msg: Message) -> Result<(), WsError> {
        match self.socket {
            // TODO: more closely investigate difference between `Sink::send` and `Sink::send_all`
            // it got something to do with batching and flushing - it might be important if it
            // turns out somehow we've got a bottleneck here
            Some(ref mut ws_stream) => ws_stream.send(msg).await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn next_websocket_request(&mut self) -> Option<Result<Message, WsError>> {
        match self.socket {
            Some(ref mut ws_stream) => ws_stream.next().await,
            None => None,
        }
    }

    async fn listen_for_requests(&mut self, mut msg_receiver: ReconstructedMessagesReceiver) {
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
                }
                mix_messages = msg_receiver.next() => {
                    let mix_messages = mix_messages.expect(
                        "mix messages sender was unexpectedly closed! this shouldn't have ever happened!",
                    );
                    if let Err(e) = self.push_websocket_received_plaintexts(mix_messages).await {
                        warn!("failed to send sphinx packets back to the client - {:?}, assuming the connection is dead", e);
                        break;
                    }
                }
            }
        }
    }

    // consume self to make sure `drop` is called after this is done
    pub(crate) async fn handle_connection(mut self, socket: TcpStream) {
        let ws_stream = match accept_async(socket).await {
            Ok(ws_stream) => ws_stream,
            Err(err) => {
                warn!("error while performing the websocket handshake - {:?}", err);
                return;
            }
        };
        self.socket = Some(ws_stream);

        let (reconstructed_sender, reconstructed_receiver) = mpsc::unbounded();

        // tell the buffer to start sending stuff to us
        self.buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverAnnounce(
                reconstructed_sender,
            ))
            .expect("the buffer request failed!");

        self.listen_for_requests(reconstructed_receiver).await;
    }
}
