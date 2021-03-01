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

use client_core::client::{
    inbound_messages::{InputMessage, InputMessageSender},
    received_buffer::{
        ReceivedBufferMessage, ReceivedBufferRequestSender, ReconstructedMessagesReceiver,
    },
};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::receiver::ReconstructedMessage;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{protocol::Message as WsMessage, Error as WsError},
    WebSocketStream,
};
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

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
            self_full_address: self.self_full_address,
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

    fn handle_send(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        with_reply_surb: bool,
    ) -> Option<ServerResponse> {
        // the ack control is now responsible for chunking, etc.
        let input_msg = InputMessage::new_fresh(recipient, message, with_reply_surb);
        self.msg_input.unbounded_send(input_msg).unwrap();

        None
    }

    fn handle_reply(&mut self, reply_surb: ReplySurb, message: Vec<u8>) -> Option<ServerResponse> {
        if message.len() > ReplySurb::max_msg_len(Default::default()) {
            return Some(ServerResponse::new_error(format!("too long message to put inside a reply SURB. Received: {} bytes and maximum is {} bytes", message.len(), ReplySurb::max_msg_len(Default::default()))));
        }

        let input_msg = InputMessage::new_reply(reply_surb, message);
        self.msg_input.unbounded_send(input_msg).unwrap();

        None
    }

    fn handle_self_address(&self) -> ServerResponse {
        ServerResponse::SelfAddress(self.self_full_address)
    }

    fn handle_request(&mut self, request: ClientRequest) -> Option<ServerResponse> {
        match request {
            ClientRequest::Send {
                recipient,
                message,
                with_reply_surb,
            } => self.handle_send(recipient, message, with_reply_surb),
            ClientRequest::Reply {
                message,
                reply_surb,
            } => self.handle_reply(reply_surb, message),
            ClientRequest::SelfAddress => Some(self.handle_self_address()),
        }
    }

    fn handle_text_message(&mut self, msg: String) -> Option<WsMessage> {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg);

        self.received_response_type = ReceivedResponseType::Text;
        let client_request = ClientRequest::try_from_text(msg);

        let response = match client_request {
            Err(err) => Some(ServerResponse::Error(err)),
            Ok(req) => self.handle_request(req),
        };

        response.map(|resp| WsMessage::text(resp.into_text()))
    }

    fn handle_binary_message(&mut self, msg: Vec<u8>) -> Option<WsMessage> {
        debug!("Handling binary message request");

        self.received_response_type = ReceivedResponseType::Binary;
        let client_request = ClientRequest::try_from_binary(msg);

        let response = match client_request {
            Err(err) => Some(ServerResponse::Error(err)),
            Ok(req) => self.handle_request(req),
        };

        response.map(|resp| WsMessage::Binary(resp.into_binary()))
    }

    fn handle_ws_request(&mut self, raw_request: WsMessage) -> Option<WsMessage> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // old version of this file.
        match raw_request {
            WsMessage::Text(text_message) => self.handle_text_message(text_message),
            WsMessage::Binary(binary_message) => self.handle_binary_message(binary_message),
            _ => None,
        }
    }

    // I'm still not entirely sure why `send_all` requires `TryStream` rather than `Stream`, but
    // let's just play along for now
    fn prepare_reconstructed_binary(
        &self,
        reconstructed_messages: Vec<ReconstructedMessage>,
    ) -> Vec<Result<WsMessage, WsError>> {
        reconstructed_messages
            .into_iter()
            .map(ServerResponse::Received)
            .map(|resp| Ok(WsMessage::Binary(resp.into_binary())))
            .collect()
    }

    // I'm still not entirely sure why `send_all` requires `TryStream` rather than `Stream`, but
    // let's just play along for now
    fn prepare_reconstructed_text(
        &self,
        reconstructed_messages: Vec<ReconstructedMessage>,
    ) -> Vec<Result<WsMessage, WsError>> {
        reconstructed_messages
            .into_iter()
            .map(ServerResponse::Received)
            .map(|resp| Ok(WsMessage::Text(resp.into_text())))
            .collect()
    }

    async fn push_websocket_received_plaintexts(
        &mut self,
        reconstructed_messages: Vec<ReconstructedMessage>,
    ) -> Result<(), WsError> {
        // TODO: later there might be a flag on the reconstructed message itself to tell us
        // if it's text or binary, but for time being we use the naive assumption that if
        // client is sending Message::Text it expects text back. Same for Message::Binary
        let response_messages = match self.received_response_type {
            ReceivedResponseType::Binary => {
                self.prepare_reconstructed_binary(reconstructed_messages)
            }
            ReceivedResponseType::Text => self.prepare_reconstructed_text(reconstructed_messages),
        };

        let mut send_stream = futures::stream::iter(response_messages);
        self.socket
            .as_mut()
            .unwrap()
            .send_all(&mut send_stream)
            .await
    }

    async fn send_websocket_response(&mut self, msg: WsMessage) -> Result<(), WsError> {
        match self.socket {
            // TODO: more closely investigate difference between `Sink::send` and `Sink::send_all`
            // it got something to do with batching and flushing - it might be important if it
            // turns out somehow we've got a bottleneck here
            Some(ref mut ws_stream) => ws_stream.send(msg).await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn next_websocket_request(&mut self) -> Option<Result<WsMessage, WsError>> {
        match self.socket {
            Some(ref mut ws_stream) => ws_stream.next().await,
            None => None,
        }
    }

    async fn listen_for_requests(&mut self, mut msg_receiver: ReconstructedMessagesReceiver) {
        loop {
            tokio::select! {
                // we can either get a client request from the websocket
                socket_msg = self.next_websocket_request() => {
                    if socket_msg.is_none() {
                        break;
                    }
                    let socket_msg = match socket_msg.unwrap() {
                        Ok(socket_msg) => socket_msg,
                        Err(err) => {
                            warn!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_ws_request(socket_msg) {
                        if let Err(err) = self.send_websocket_response(response).await {
                            warn!(
                                "Failed to send message over websocket: {}. Assuming the connection is dead.",
                                err
                            );
                            break;
                        }
                    }
                }
                // or a reconstructed mix message that we need to push back to the client
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
