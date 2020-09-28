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

use crate::cleanup_socket_message;
use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
use futures::channel::oneshot;
use futures::stream::{SplitSink, SplitStream};
use futures::{FutureExt, SinkExt, StreamExt};
use gateway_requests::registration::handshake::SharedKeys;
use gateway_requests::BinaryResponse;
use log::*;
use std::sync::Arc;
use tungstenite::Message;

#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::WebSocketStream;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

// type alias for not having to type the whole thing every single time (and now it makes it easier
// to use different types based on compilation target)
#[cfg(not(target_arch = "wasm32"))]
type WsConn = WebSocketStream<TcpStream>;

#[cfg(target_arch = "wasm32")]
type WsConn = JSWebsocket;

// We have ownership over sink half of the connection, but the stream is owned
// by some other task, however, we can notify it to get the stream back.

type SplitStreamReceiver = oneshot::Receiver<Result<SplitStream<WsConn>, GatewayClientError>>;

pub(crate) struct PartiallyDelegated {
    sink_half: SplitSink<WsConn, Message>,
    delegated_stream: (SplitStreamReceiver, oneshot::Sender<()>),
}

impl PartiallyDelegated {
    fn route_socket_message(
        ws_msg: Message,
        packet_router: &PacketRouter,
        shared_key: &SharedKeys,
    ) {
        match ws_msg {
            Message::Binary(bin_msg) => {
                // this function decrypts the request and checks the MAC
                let plaintext =
                    match BinaryResponse::try_from_encrypted_tagged_bytes(bin_msg, shared_key) {
                        Ok(bin_response) => match bin_response {
                            BinaryResponse::PushedMixMessage(plaintext) => plaintext,
                        },
                        Err(err) => {
                            warn!(
                                "message received from the gateway was malformed! - {:?}",
                                err
                            );
                            return;
                        }
                    };

                // TODO: some batching mechanism to allow reading and sending more than
                // one packet at the time, because the receiver can easily handle it
                packet_router.route_received(vec![plaintext])
            }
            // I think that in the future we should perhaps have some sequence number system, i.e.
            // so each request/response pair can be easily identified, so that if messages are
            // not ordered (for some peculiar reason) we wouldn't lose anything.
            // This would also require NOT discarding any text responses here.

            // TODO: those can return the "send confirmations" - perhaps it should be somehow worked around?
            Message::Text(text) => debug!(
                "received a text message - probably a response to some previous query! - {}",
                text
            ),
            _ => (),
        };
    }

    pub(crate) fn split_and_listen_for_mixnet_messages(
        conn: WsConn,
        packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
    ) -> Result<Self, GatewayClientError> {
        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        let (notify_sender, notify_receiver) = oneshot::channel();
        let (stream_sender, stream_receiver) = oneshot::channel();

        let (sink, mut stream) = conn.split();

        let mixnet_receiver_future = async move {
            let mut fused_receiver = notify_receiver.fuse();
            let mut fused_stream = (&mut stream).fuse();

            let ret_err = loop {
                futures::select! {
                    _ = fused_receiver => {
                        break Ok(());
                    }
                    msg = fused_stream.next() => {
                        let ws_msg = match cleanup_socket_message(msg) {
                            Err(err) => break Err(err),
                            Ok(msg) => msg
                        };
                        Self::route_socket_message(ws_msg, &packet_router, shared_key.as_ref());
                    }
                };
            };

            match ret_err {
                Err(err) => stream_sender.send(Err(err)),
                Ok(_) => stream_sender.send(Ok(stream)),
            }
            .unwrap();
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(mixnet_receiver_future);

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(mixnet_receiver_future);

        Ok(PartiallyDelegated {
            sink_half: sink,
            delegated_stream: (stream_receiver, notify_sender),
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

    pub(crate) async fn batch_send_without_response(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<(), GatewayClientError> {
        let stream_messages: Vec<_> = messages.into_iter().map(Ok).collect();
        let mut send_stream = futures::stream::iter(stream_messages);
        Ok(self.sink_half.send_all(&mut send_stream).await?)
    }

    pub(crate) async fn merge(self) -> Result<WsConn, GatewayClientError> {
        let (stream_receiver, notify) = self.delegated_stream;
        notify.send(()).unwrap();
        let stream = stream_receiver.await.unwrap()?;
        // the error is thrown when trying to reunite sink and stream that did not originate
        // from the same split which is impossible to happen here
        Ok(self.sink_half.reunite(stream).unwrap())
    }
}

// we can either have the stream itself or an option to re-obtain it
// by notifying the future owning it to finish the execution and awaiting the result
// which should be almost immediate (or an invalid state which should never, ever happen)
pub(crate) enum SocketState {
    Available(WsConn),
    PartiallyDelegated(PartiallyDelegated),
    NotConnected,
    Invalid,
}

impl SocketState {
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
