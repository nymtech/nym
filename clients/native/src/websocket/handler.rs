// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_connections::{
    ConnectionCommand, ConnectionCommandSender, LaneQueueLengths, TransmissionLane,
};
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
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
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
    client_connection_tx: ConnectionCommandSender,
    buffer_requester: ReceivedBufferRequestSender,
    self_full_address: Recipient,
    socket: Option<WebSocketStream<TcpStream>>,
    received_response_type: ReceivedResponseType,
    lane_queue_lengths: LaneQueueLengths,
}

// clone is used to use handler on a new connection, which initially is `None`
impl Clone for Handler {
    fn clone(&self) -> Self {
        Handler {
            msg_input: self.msg_input.clone(),
            client_connection_tx: self.client_connection_tx.clone(),
            buffer_requester: self.buffer_requester.clone(),
            self_full_address: self.self_full_address,
            socket: None,
            received_response_type: Default::default(),
            lane_queue_lengths: self.lane_queue_lengths.clone(),
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        if self
            .buffer_requester
            .unbounded_send(ReceivedBufferMessage::ReceiverDisconnect)
            .is_err()
        {
            error!("we failed to disconnect the receiver from the buffer! presumably the shutdown procedure has been initiated!")
        }
    }
}

impl Handler {
    pub(crate) fn new(
        msg_input: InputMessageSender,
        client_connection_tx: ConnectionCommandSender,
        buffer_requester: ReceivedBufferRequestSender,
        self_full_address: Recipient,
        lane_queue_lengths: LaneQueueLengths,
    ) -> Self {
        Handler {
            msg_input,
            client_connection_tx,
            buffer_requester,
            self_full_address,
            socket: None,
            received_response_type: Default::default(),
            lane_queue_lengths,
        }
    }

    async fn handle_send(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        connection_id: Option<u64>,
    ) -> Option<ServerResponse> {
        info!(
            "Attempting to send {:.2} kiB message to {recipient} on connection_id {connection_id:?}",
            message.len() as f64 / 1024.0
        );

        // We map the absence of a connection id as going into the general lane.
        let lane = connection_id.map_or(TransmissionLane::General, |id| {
            TransmissionLane::ConnectionId(id)
        });

        // the ack control is now responsible for chunking, etc.
        let input_msg = InputMessage::new_regular(recipient, message, lane);
        self.msg_input
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");

        // Only reply back with a `LaneQueueLength` if the sender providided a connection id
        let connection_id = match lane {
            TransmissionLane::General
            | TransmissionLane::ReplySurbRequest
            | TransmissionLane::Retransmission
            | TransmissionLane::AdditionalReplySurbs => return None,
            TransmissionLane::ConnectionId(id) => id,
        };

        // on receiving a send, we reply back the current lane queue length for that connection id.
        // Note that this does _NOT_ take into account the packets that have been received but not
        // yet reach `OutQueueControl`, so it might be a tad low.
        if let Ok(lane_queue_lengths) = self.lane_queue_lengths.lock() {
            let queue_length = lane_queue_lengths.get(&lane).unwrap_or(0);
            return Some(ServerResponse::LaneQueueLength {
                lane: connection_id,
                queue_length,
            });
        }

        log::warn!("Failed to get the lane queue length lock, not responding back with the current queue length");
        None
    }

    async fn handle_send_anonymous(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
        reply_surbs: u32,
        connection_id: Option<u64>,
    ) -> Option<ServerResponse> {
        info!(
            "Attempting to anonymously send {:.2} kiB message to {recipient} on connection_id {connection_id:?} while attaching {reply_surbs} replySURBs.",
            message.len() as f64 / 1024.0
        );

        // We map the absence of a connection id as going into the general lane.
        let lane = connection_id.map_or(TransmissionLane::General, |id| {
            TransmissionLane::ConnectionId(id)
        });

        let input_msg = InputMessage::new_anonymous(recipient, message, reply_surbs, lane);
        self.msg_input
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");

        // Only reply back with a `LaneQueueLength` if the sender providided a connection id
        let connection_id = match lane {
            TransmissionLane::General
            | TransmissionLane::ReplySurbRequest
            | TransmissionLane::Retransmission
            | TransmissionLane::AdditionalReplySurbs => return None,
            TransmissionLane::ConnectionId(id) => id,
        };

        // on receiving a send, we reply back the current lane queue length for that connection id.
        // Note that this does _NOT_ take into account the packets that have been received but not
        // yet reach `OutQueueControl`, so it might be a tad low.
        if let Ok(lane_queue_lengths) = self.lane_queue_lengths.lock() {
            let queue_length = lane_queue_lengths.get(&lane).unwrap_or(0);
            return Some(ServerResponse::LaneQueueLength {
                lane: connection_id,
                queue_length,
            });
        }

        log::warn!("Failed to get the lane queue length lock, not responding back with the current queue length");
        None
    }

    async fn handle_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        message: Vec<u8>,
        connection_id: Option<u64>,
    ) -> Option<ServerResponse> {
        info!("Attempting to send {:.2} kiB reply message to {recipient_tag} on connection_id {connection_id:?}", message.len() as f64 / 1024.0);

        // We map the absence of a connection id as going into the general lane.
        let lane = connection_id.map_or(TransmissionLane::General, |id| {
            TransmissionLane::ConnectionId(id)
        });

        let input_msg = InputMessage::new_reply(recipient_tag, message, lane);
        self.msg_input
            .send(input_msg)
            .await
            .expect("InputMessageReceiver has stopped receiving!");

        // Only reply back with a `LaneQueueLength` if the sender providided a connection id
        let connection_id = match lane {
            TransmissionLane::General
            | TransmissionLane::ReplySurbRequest
            | TransmissionLane::Retransmission
            | TransmissionLane::AdditionalReplySurbs => return None,
            TransmissionLane::ConnectionId(id) => id,
        };

        // on receiving a send, we reply back the current lane queue length for that connection id.
        // Note that this does _NOT_ take into account the packets that have been received but not
        // yet reach `OutQueueControl`, so it might be a tad low.
        if let Ok(lane_queue_lengths) = self.lane_queue_lengths.lock() {
            let queue_length = lane_queue_lengths.get(&lane).unwrap_or(0);
            return Some(ServerResponse::LaneQueueLength {
                lane: connection_id,
                queue_length,
            });
        }

        log::warn!("Failed to get the lane queue length lock, not responding back with the current queue length");
        None
    }

    fn handle_self_address(&self) -> ServerResponse {
        ServerResponse::SelfAddress(Box::new(self.self_full_address))
    }

    fn handle_closed_connection(&self, connection_id: u64) -> Option<ServerResponse> {
        self.client_connection_tx
            .unbounded_send(ConnectionCommand::Close(connection_id))
            .unwrap();
        None
    }

    fn handle_get_lane_queue_length(&self, connection_id: u64) -> Option<ServerResponse> {
        let Ok(lane_queue_lengths) = self.lane_queue_lengths.lock() else {
            log::warn!(
                "Failed to get the lane queue length lock, not responding back with the current queue length"
            );
            return None;
        };

        let lane = TransmissionLane::ConnectionId(connection_id);
        let queue_length = lane_queue_lengths.get(&lane).unwrap_or(0);
        Some(ServerResponse::LaneQueueLength {
            lane: connection_id,
            queue_length,
        })
    }

    async fn handle_request(&mut self, request: ClientRequest) -> Option<ServerResponse> {
        match request {
            ClientRequest::Send {
                recipient,
                message,
                connection_id,
            } => self.handle_send(recipient, message, connection_id).await,

            ClientRequest::SendAnonymous {
                recipient,
                message,
                reply_surbs,
                connection_id,
            } => {
                self.handle_send_anonymous(recipient, message, reply_surbs, connection_id)
                    .await
            }

            ClientRequest::Reply {
                message,
                sender_tag,
                connection_id,
            } => self.handle_reply(sender_tag, message, connection_id).await,

            ClientRequest::SelfAddress => Some(self.handle_self_address()),
            ClientRequest::ClosedConnection(id) => self.handle_closed_connection(id),
            ClientRequest::GetLaneQueueLength(id) => self.handle_get_lane_queue_length(id),
        }
    }

    async fn handle_text_message(&mut self, msg: String) -> Option<WsMessage> {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg);

        self.received_response_type = ReceivedResponseType::Text;
        let client_request = ClientRequest::try_from_text(msg);

        let response = match client_request {
            Err(err) => Some(ServerResponse::Error(err)),
            Ok(req) => self.handle_request(req).await,
        };

        response.map(|resp| WsMessage::text(resp.into_text()))
    }

    async fn handle_binary_message(&mut self, msg: &[u8]) -> Option<WsMessage> {
        debug!("Handling binary message request");

        self.received_response_type = ReceivedResponseType::Binary;
        let client_request = ClientRequest::try_from_binary(msg);

        let response = match client_request {
            Err(err) => Some(ServerResponse::Error(err)),
            Ok(req) => self.handle_request(req).await,
        };

        response.map(|resp| WsMessage::Binary(resp.into_binary()))
    }

    async fn handle_ws_request(&mut self, raw_request: WsMessage) -> Option<WsMessage> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // old version of this file.
        match raw_request {
            WsMessage::Text(text_message) => self.handle_text_message(text_message).await,
            WsMessage::Binary(binary_message) => self.handle_binary_message(&binary_message).await,
            _ => None,
        }
    }

    async fn push_websocket_received_plaintexts(
        &mut self,
        reconstructed_messages: Vec<ReconstructedMessage>,
    ) -> Result<(), WsError> {
        // TODO: later there might be a flag on the reconstructed message itself to tell us
        // if it's text or binary, but for time being we use the naive assumption that if
        // client is sending Message::Text it expects text back. Same for Message::Binary
        let response_messages = match self.received_response_type {
            ReceivedResponseType::Binary => prepare_reconstructed_binary(reconstructed_messages),
            ReceivedResponseType::Text => prepare_reconstructed_text(reconstructed_messages),
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
                            warn!("failed to obtain message from websocket stream! stopping connection handler: {err}");
                            break;
                        }
                    };

                    if socket_msg.is_close() {
                        break;
                    }

                    if let Some(response) = self.handle_ws_request(socket_msg).await {
                        if let Err(err) = self.send_websocket_response(response).await {
                            warn!(
                                "Failed to send message over websocket: {err}. Assuming the connection is dead.",
                            );
                            break;
                        }
                    }
                }
                // or a reconstructed mix message that we need to push back to the client
                mix_messages = msg_receiver.next() => {
                    let Some(mix_messages) = mix_messages else {
                        error!("mix messages sender was unexpectedly closed! this shouldn't have ever happened! (unless we're shutting down - TODO: implement proper graceful shutdown handler)");
                        return
                    };
                    if let Err(err) = self.push_websocket_received_plaintexts(mix_messages).await {
                        warn!("failed to send sphinx packets back to the client - {err}, assuming the connection is dead");
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
                warn!("error while performing the websocket handshake - {err}");
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

// I'm still not entirely sure why `send_all` requires `TryStream` rather than `Stream`, but
// let's just play along for now
fn prepare_reconstructed_binary(
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
    reconstructed_messages: Vec<ReconstructedMessage>,
) -> Vec<Result<WsMessage, WsError>> {
    reconstructed_messages
        .into_iter()
        .map(ServerResponse::Received)
        .map(|resp| Ok(WsMessage::Text(resp.into_text())))
        .collect()
}
