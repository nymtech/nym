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

use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
use crate::read_ws_stream_message;
use futures::stream::{SplitSink, SplitStream};
use futures::{future::BoxFuture, FutureExt, SinkExt, StreamExt};
use gateway_requests::registration::handshake::SharedKeys;
use gateway_requests::BinaryResponse;
use log::*;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

// type alias for not having to type the whole thing every single time
type WsConn = WebSocketStream<TcpStream>;

// We have ownership over sink half of the connection, but the stream is owned
// by some other task, however, we can notify it to get the stream back.
pub(crate) struct PartiallyDelegated<'a> {
    sink_half: SplitSink<WsConn, Message>,
    delegated_stream: (
        BoxFuture<'a, Result<SplitStream<WsConn>, GatewayClientError>>,
        Arc<Notify>,
    ),
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> PartiallyDelegated<'a> {
    // TODO: this can be potentially bad as we have no direct restrictions of ensuring it's called
    // within tokio runtime. Perhaps we should use the "old" way of passing explicit
    // runtime handle to the constructor and using that instead?
    pub(crate) fn split_and_listen_for_mixnet_messages(
        conn: WsConn,
        packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
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
                    msg = read_ws_stream_message(&mut stream) => {
                        match msg? {
                            Message::Binary(bin_msg) => {
                                // this function decrypts the request and checks the MAC
                                let plaintext = match BinaryResponse::try_from_encrypted_tagged_bytes(bin_msg, shared_key.as_ref()) {
                                    Ok(bin_response) => match bin_response {
                                        BinaryResponse::PushedMixMessage(plaintext) => plaintext,
                                    },
                                    Err(err) => {
                                        warn!("message received from the gateway was malformed! - {:?}", err);
                                        continue
                                    }
                                };

                                // TODO: some batching mechanism to allow reading and sending more than
                                // one packet at the time, because the receiver can easily handle it
                                packet_router.route_received(vec![plaintext])
                            },
                            // I think that in the future we should perhaps have some sequence number system, i.e.
                            // so each request/response pair can be easily identified, so that if messages are
                            // not ordered (for some peculiar reason) we wouldn't lose anything.
                            // This would also require NOT discarding any text responses here.

                            // TODO: those can return the "send confirmations" - perhaps it should be somehow worked around?
                            Message::Text(text) => debug!("received a text message - probably a response to some previous query! - {}", text),
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
    pub(crate) async fn send_without_response(
        &mut self,
        msg: Message,
    ) -> Result<(), GatewayClientError> {
        Ok(self.sink_half.send(msg).await?)
    }

    pub(crate) async fn merge(self) -> Result<WsConn, GatewayClientError> {
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
pub(crate) enum SocketState<'a> {
    Available(WsConn),
    PartiallyDelegated(PartiallyDelegated<'a>),
    NotConnected,
    Invalid,
}

impl<'a> SocketState<'a> {
    pub(crate) fn is_available(&self) -> bool {
        match self {
            SocketState::Available(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_partially_delegated(&self) -> bool {
        match self {
            SocketState::PartiallyDelegated(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_established(&self) -> bool {
        match self {
            SocketState::Available(_) | SocketState::PartiallyDelegated(_) => true,
            _ => false,
        }
    }
}
