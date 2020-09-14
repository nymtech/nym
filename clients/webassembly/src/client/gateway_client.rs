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

#[macro_use]
use crate::{console_log, console_warn};
use crate::websocket::JSWebsocket;
use crate::DEFAULT_RNG;
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gateway_client::error::GatewayClientError;
use gateway_client::packet_router::PacketRouter;
use gateway_requests::registration::handshake::{client_handshake, SharedKeys};
use nymsphinx::addressing::nodes::NodeIdentity;
use std::sync::Arc;
use topology::gateway;
use tungstenite::{Error as WsError, Message as WsMessage};

// We have ownership over sink half of the connection, but the stream is owned
// by some other task, however, we can notify it to get the stream back.
struct PartiallyDelegated<'a> {
    sink_half: SplitSink<JSWebsocket, WsMessage>,
    delegated_stream: (
        BoxFuture<'a, Result<SplitStream<JSWebsocket>, GatewayClientError>>,
        Arc<oneshot::Sender<()>>,
    ),
}

impl<'a> PartiallyDelegated<'a> {
    fn split_and_listen_for_mixnet_messages(
        conn: JSWebsocket,
        packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
    ) -> Result<Self, GatewayClientError> {
        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        // let (tx, rx) = oneshot::channel();
        // let (sink, mut stream) = conn.split();
        //
        // let mixnet_receiver_future = async move {
        //     let mut should_return = false;
        //     while !should_return {
        //         tokio::select! {
        //             _ = notify_clone.notified() => {
        //                 should_return = true;
        //             }
        //             msg = read_ws_stream_message(&mut stream) => {
        //                 match msg? {
        //                     Message::Binary(bin_msg) => {
        //                         // this function decrypts the request and checks the MAC
        //                         let plaintext = match BinaryResponse::try_from_encrypted_tagged_bytes(bin_msg, shared_key.as_ref()) {
        //                             Ok(bin_response) => match bin_response {
        //                                 BinaryResponse::PushedMixMessage(plaintext) => plaintext,
        //                             },
        //                             Err(err) => {
        //                                 warn!("message received from the gateway was malformed! - {:?}", err);
        //                                 continue
        //                             }
        //                         };
        //
        //                         // TODO: some batching mechanism to allow reading and sending more than
        //                         // one packet at the time, because the receiver can easily handle it
        //                         packet_router.route_received(vec![plaintext])
        //                     },
        //                     // I think that in the future we should perhaps have some sequence number system, i.e.
        //                     // so each request/response pair can be easily identified, so that if messages are
        //                     // not ordered (for some peculiar reason) we wouldn't lose anything.
        //                     // This would also require NOT discarding any text responses here.
        //
        //                     // TODO: those can return the "send confirmations" - perhaps it should be somehow worked around?
        //                     Message::Text(text) => debug!("received a text message - probably a response to some previous query! - {}", text),
        //                     _ => (),
        //                 };
        //             }
        //         };
        //     }
        //     Ok(stream)
        // };
        //
        // let spawned_boxed_task = tokio::spawn(mixnet_receiver_future)
        //     .map(|join_handle| {
        //         join_handle.expect("task must have not failed to finish its execution!")
        //     })
        //     .boxed();
        //
        // Ok(PartiallyDelegated {
        //     sink_half: sink,
        //     delegated_stream: (spawned_boxed_task, notify),
        // })

        todo!()
    }

    // if we want to send a message and don't care about response, we can don't need to reunite the split,
    // the sink itself is enough
    async fn send_without_response(&mut self, msg: WsMessage) -> Result<(), GatewayClientError> {
        todo!()
        // Ok(self.sink_half.send(msg).await?)
    }

    async fn merge(self) -> Result<JSWebsocket, GatewayClientError> {
        let (stream_fut, notify) = self.delegated_stream;

        todo!("notify was here");

        let stream = stream_fut.await?;
        // the error is thrown when trying to reunite sink and stream that did not originate
        // from the same split which is impossible to happen here
        Ok(self.sink_half.reunite(stream).unwrap())
    }
}

// we can either have the stream itself or an option to re-obtain it
// by notifying the future owning it to finish the execution and awaiting the result
// which should be almost immediate (or an invalid state which should never, ever happen)
enum SocketState<'a> {
    Available(JSWebsocket),
    PartiallyDelegated(PartiallyDelegated<'a>),
    NotConnected,
    Invalid,
}

// TODO: eventually remove this in favour of making gateway_client::GatewayClient
// wasm-compatible and compilable
pub(super) struct GatewayClient {
    gateway_identity: NodeIdentity,
    shared_keys: SharedKeys,
    socket: JSWebsocket,
}

impl GatewayClient {
    pub(super) async fn establish_relation(
        gateway: &gateway::Node,
        local_identity: Arc<identity::KeyPair>,
    ) -> Result<Self, GatewayClientError> {
        // 1. create connection
        let address = gateway.client_listener.as_ref();
        let gateway_identity = gateway.identity_key;
        let mut socket = JSWebsocket::new(address)?;

        // 2. derive key
        let shared_keys = client_handshake(
            &mut DEFAULT_RNG,
            &mut socket,
            &local_identity,
            gateway_identity,
        )
        .await
        .expect("failed to perform gateway handshake!");

        console_log!(
            "Established shared keys {:?} with gateway: {:?}",
            shared_keys,
            gateway_identity
        );

        Ok(Self {
            gateway_identity,
            shared_keys,
            socket,
        })
    }

    pub(super) fn gateway_identity(&self) -> NodeIdentity {
        self.gateway_identity
    }
}
