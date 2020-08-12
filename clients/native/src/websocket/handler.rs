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

use crate::client::{
    inbound_messages::{InputMessage, InputMessageSender},
    received_buffer::{
        ReceivedBufferMessage, ReceivedBufferRequestSender, ReconstructedMessagesReceiver,
    },
};
use crate::websocket::api;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySURB;
use nymsphinx::receiver::ReconstructedMessage;
use std::convert::{TryFrom, TryInto};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{protocol::Message, Error as WsError},
    WebSocketStream,
};

enum ReceivedResponseType {
    Binary,
    Text,
}

impl Default for ReceivedResponseType {
    fn default() -> Self {
        ReceivedResponseType::Binary
    }
}

pub(crate) struct Handler {
    msg_input: InputMessageSender,
    buffer_requester: ReceivedBufferRequestSender,
    self_full_address: Recipient,
    socket: Option<WebSocketStream<TcpStream>>,
    received_response_type: ReceivedResponseType,
}

// clone is used to use handler on a new connection, which initially is `None`
impl Clone for Handler {
    fn clone(&self) -> Self {
        Handler {
            msg_input: self.msg_input.clone(),
            buffer_requester: self.buffer_requester.clone(),
            self_full_address: self.self_full_address.clone(),
            socket: None,
            received_response_type: Default::default(),
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
            .expect("the buffer request failed!")
    }
}

impl Handler {
    pub(crate) fn new(
        msg_input: InputMessageSender,
        buffer_requester: ReceivedBufferRequestSender,
        self_full_address: Recipient,
    ) -> Self {
        Handler {
            msg_input,
            buffer_requester,
            self_full_address,
            socket: None,
            received_response_type: Default::default(),
        }
    }

    fn handle_text_send(
        &mut self,
        msg: String,
        full_recipient_address: String,
        with_reply_surb: bool,
    ) -> Option<api::text::ServerResponse> {
        let message_bytes = msg.into_bytes();

        let recipient = match Recipient::try_from_string(full_recipient_address) {
            Ok(address) => address,
            Err(err) => {
                trace!("failed to parse received Recipient: {:?}", err);
                return Some(api::text::ServerResponse::new_error(
                    "malformed recipient address",
                ));
            }
        };

        // the ack control is now responsible for chunking, etc.
        let input_msg = InputMessage::new_fresh(recipient, message_bytes, with_reply_surb);
        self.msg_input.unbounded_send(input_msg).unwrap();

        self.received_response_type = ReceivedResponseType::Text;
        None
    }

    fn handle_text_reply(
        &mut self,
        msg: String,
        reply_surb: String,
    ) -> Option<api::text::ServerResponse> {
        let message_bytes = msg.into_bytes();

        let reply_surb = match ReplySURB::from_base58_string(reply_surb) {
            Ok(reply_surb) => reply_surb,
            Err(err) => {
                trace!("failed to parse received ReplySURB: {:?}", err);
                return Some(api::text::ServerResponse::new_error("malformed reply surb"));
            }
        };

        let input_msg = InputMessage::new_reply(reply_surb, message_bytes);
        self.msg_input.unbounded_send(input_msg).unwrap();

        self.received_response_type = ReceivedResponseType::Text;
        None
    }

    fn handle_text_self_address(&self) -> api::text::ServerResponse {
        api::text::ServerResponse::SelfAddress {
            address: self.self_full_address.to_string(),
        }
    }

    async fn handle_text_message(&mut self, msg: String) -> Option<Message> {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg.clone());

        match api::text::ClientRequest::try_from(msg) {
            Err(e) => Some(
                api::text::ServerResponse::Error {
                    message: format!("received invalid request. err: {:?}", e),
                }
                .into(),
            ),
            Ok(req) => match req {
                api::text::ClientRequest::Send {
                    message,
                    recipient,
                    with_reply_surb,
                } => self
                    .handle_text_send(message, recipient, with_reply_surb)
                    .map(|resp| resp.into()),
                api::text::ClientRequest::Reply {
                    message,
                    reply_surb,
                } => self
                    .handle_text_reply(message, reply_surb)
                    .map(|resp| resp.into()),
                api::text::ClientRequest::SelfAddress => {
                    Some(self.handle_text_self_address().into())
                }
            },
        }
    }

    fn handle_binary_send(
        &mut self,
        recipient: Recipient,
        data: Vec<u8>,
        with_reply_surb: bool,
    ) -> Option<api::binary::ServerResponse> {
        // the ack control is now responsible for chunking, etc.
        let input_msg = InputMessage::new_fresh(recipient, data, with_reply_surb);
        self.msg_input.unbounded_send(input_msg).unwrap();

        self.received_response_type = ReceivedResponseType::Binary;

        None
    }

    fn handle_binary_reply(
        &mut self,
        reply_surb: ReplySURB,
        message: Vec<u8>,
    ) -> Option<api::binary::ServerResponse> {
        if message.len() > ReplySURB::max_msg_len(Default::default()) {
            return Some(api::binary::ServerResponse::new_error(format!("too long message to put inside a reply SURB. Received: {} bytes and maximum is {} bytes", message.len(), ReplySURB::max_msg_len(Default::default()))));
        }

        let input_msg = InputMessage::new_reply(reply_surb, message);
        self.msg_input.unbounded_send(input_msg).unwrap();

        self.received_response_type = ReceivedResponseType::Binary;

        None
    }

    fn handle_binary_self_address(&self) -> api::binary::ServerResponse {
        todo!()
    }

    // if it's binary we assume it's a sphinx packet formatted the same way as we'd have sent
    // it to the gateway
    fn handle_binary_message(&mut self, msg: Vec<u8>) -> Option<Message> {
        debug!("Handling binary message request");

        self.received_response_type = ReceivedResponseType::Binary;
        // make sure it is correctly formatted
        let binary_request = match api::binary::ClientRequest::deserialize(&msg) {
            Ok(bin_request) => bin_request,
            Err(err) => return Some(api::binary::ServerResponse::Error(err).into()),
        };

        match binary_request {
            api::binary::ClientRequest::Send {
                recipient,
                data,
                with_reply_surb,
            } => self.handle_binary_send(recipient, data, with_reply_surb),
            api::binary::ClientRequest::Reply {
                message,
                reply_surb,
            } => self.handle_binary_reply(reply_surb, message),
            api::binary::ClientRequest::SelfAddress => Some(self.handle_binary_self_address()),
        }
        .map(|resp| resp.into())
    }

    async fn handle_request(&mut self, raw_request: Message) -> Option<Message> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // old version of this file.
        match raw_request {
            Message::Text(text_message) => self.handle_text_message(text_message).await,
            Message::Binary(binary_message) => self.handle_binary_message(binary_message),
            _ => None,
        }
    }

    async fn push_websocket_received_plaintexts(
        &mut self,
        reconstructed_messages: Vec<ReconstructedMessage>,
    ) -> Result<(), WsError> {
        // TODO: later there might be a flag on the reconstructed message itself

        let response_messages: Vec<_> = match self.received_response_type {
            ReceivedResponseType::Binary => reconstructed_messages
                .into_iter()
                .map(|msg| Ok(Message::Binary(msg.into_bytes())))
                .collect(),
            ReceivedResponseType::Text => {
                let mut decoded_messages: Vec<api::text::ReceivedTextMessage> = Vec::new();
                // either all succeed or all fall back
                let mut did_fail = false;
                for message in reconstructed_messages.iter() {
                    match message.try_into() {
                        Ok(msg) => decoded_messages.push(msg),
                        Err(err) => {
                            did_fail = true;
                            warn!("Invalid UTF-8 sequence in response message - {:?}", err);
                            break;
                        }
                    }
                }
                if did_fail {
                    reconstructed_messages
                        .into_iter()
                        .map(|msg| Ok(Message::Binary(msg.into_bytes())))
                        .collect()
                } else {
                    decoded_messages
                        .into_iter()
                        .map(|msg| Ok(api::text::ServerResponse::Received(msg).into()))
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
