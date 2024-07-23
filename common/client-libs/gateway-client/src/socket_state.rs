// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bandwidth::ClientBandwidth;
use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
use crate::traits::GatewayPacketRouter;
use crate::{cleanup_socket_messages, try_decrypt_binary_message};
use futures::channel::oneshot;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::*;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_gateway_requests::{ServerResponse, SimpleGatewayRequestsError};
use nym_task::TaskClient;
use std::os::raw::c_int as RawFd;
use std::sync::Arc;
use tungstenite::{protocol::Message, Error as WsError};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

// type alias for not having to type the whole thing every single time (and now it makes it easier
// to use different types based on compilation target)
#[cfg(not(target_arch = "wasm32"))]
type WsConn = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[cfg(target_arch = "wasm32")]
type WsConn = JSWebsocket;

// We have ownership over sink half of the connection, but the stream is owned
// by some other task, however, we can notify it to get the stream back.

type SplitStreamReceiver = oneshot::Receiver<Result<SplitStream<WsConn>, GatewayClientError>>;
type SplitStreamSender = oneshot::Sender<Result<SplitStream<WsConn>, GatewayClientError>>;

pub(crate) fn ws_fd(_conn: &WsConn) -> Option<RawFd> {
    #[cfg(unix)]
    match _conn.get_ref() {
        MaybeTlsStream::Plain(stream) => Some(stream.as_raw_fd()),
        &_ => None,
    }
    #[cfg(not(unix))]
    None
}

#[derive(Debug)]
pub(crate) struct PartiallyDelegatedHandle {
    sink_half: SplitSink<WsConn, Message>,
    // this could have been simplified by a notify as opposed to oneshot, but let's not change what ain't broke
    delegated_stream: (SplitStreamReceiver, oneshot::Sender<()>),
    ws_fd: Option<RawFd>,
}

struct PartiallyDelegatedRouter {
    packet_router: PacketRouter,
    shared_key: Arc<SharedKeys>,
    client_bandwidth: ClientBandwidth,

    stream_return: SplitStreamSender,
    stream_return_requester: oneshot::Receiver<()>,
}

impl PartiallyDelegatedRouter {
    fn new(
        packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
        client_bandwidth: ClientBandwidth,
        stream_return: SplitStreamSender,
        stream_return_requester: oneshot::Receiver<()>,
    ) -> PartiallyDelegatedRouter {
        PartiallyDelegatedRouter {
            packet_router,
            shared_key,
            client_bandwidth,
            stream_return,
            stream_return_requester,
        }
    }

    async fn run(mut self, mut split_stream: SplitStream<WsConn>, mut task_client: TaskClient) {
        let mut chunked_stream = (&mut split_stream).ready_chunks(8);
        let ret: Result<_, GatewayClientError> = loop {
            tokio::select! {
                biased;
                // received system-wide shutdown
                _ = task_client.recv() => {
                    log::trace!("GatewayClient listener: Received shutdown");
                    log::debug!("GatewayClient listener: Exiting");
                    return;
                }
                // received request to stop the task and return the stream
                _ = &mut self.stream_return_requester => {
                    log::debug!("received request to return the split ws stream");
                    break Ok(())
                }
                socket_msgs = chunked_stream.next() => {
                    if let Err(err) = self.handle_socket_messages(socket_msgs) {
                        break Err(err)
                    }
                }
            }
        };

        let return_res = match ret {
            Err(err) => self.stream_return.send(Err(err)),
            Ok(_) => {
                self.packet_router.mark_as_success();
                task_client.mark_as_success();
                self.stream_return.send(Ok(split_stream))
            }
        };

        if return_res.is_err() {
            warn!("failed to return the split stream back on the oneshot channel")
        }
    }

    fn handle_socket_messages(
        &self,
        msgs: Option<Vec<Result<Message, WsError>>>,
    ) -> Result<(), GatewayClientError> {
        let ws_msgs = cleanup_socket_messages(msgs)?;
        let plaintexts = self.recover_received_plaintexts(ws_msgs)?;
        if !plaintexts.is_empty() {
            self.packet_router.route_received(plaintexts)?
        }

        Ok(())
    }

    fn handle_binary_message(&self, binary_msg: Vec<u8>) -> Result<Vec<u8>, GatewayClientError> {
        // this function decrypts the request and checks the MAC
        match try_decrypt_binary_message(binary_msg, &self.shared_key) {
            Some(plaintext) => Ok(plaintext),
            None => {
                error!("failed to decrypt and verify received message!");
                Err(GatewayClientError::MalformedResponse)
            }
        }
    }

    // only returns an error on **critical** failures
    fn handle_text_message(&self, text: String) -> Result<(), GatewayClientError> {
        // if we fail to deserialise the response, return a hard error. we can't handle garbage
        match ServerResponse::try_from(text).map_err(|_| GatewayClientError::MalformedResponse)? {
            ServerResponse::Send {
                remaining_bandwidth,
            } => {
                self.client_bandwidth
                    .update_and_maybe_log(remaining_bandwidth);
                Ok(())
            }
            ServerResponse::Error { message } => {
                error!("[1] gateway failure: {message}");
                Err(GatewayClientError::GatewayError(message))
            }
            ServerResponse::TypedError { error } => {
                match error {
                    SimpleGatewayRequestsError::OutOfBandwidth {
                        required,
                        available,
                    } => {
                        warn!("run out of bandwidth when attempting to send the message! we got {available}B available, but needed at least {required}B to send the previous message");
                        self.client_bandwidth.update_and_log(available);
                        // UNIMPLEMENTED: we should stop sending messages until we recover bandwidth
                        Ok(())
                    } // _ => {
                      //     error!("[2] gateway failure: {error}");
                      //     Err(GatewayClientError::TypedGatewayError(error))
                      // }
                }
            }
            other => {
                let name = other.name();
                warn!("received illegal message of type '{name}' in an authenticated client");
                Ok(())
            }
        }
    }

    fn recover_received_plaintext(
        &self,
        message: Message,
    ) -> Result<Option<Vec<u8>>, GatewayClientError> {
        match message {
            Message::Binary(bin_msg) => {
                let plaintext = self.handle_binary_message(bin_msg)?;
                Ok(Some(plaintext))
            }
            // I think that in the future we should perhaps have some sequence number system, i.e.
            // so each request/response pair can be easily identified, so that if messages are
            // not ordered (for some peculiar reason) we wouldn't lose anything.
            // This would also require NOT discarding any text responses here.

            // TODO: those can return the "send confirmations" - perhaps it should be somehow worked around?
            Message::Text(text) => {
                trace!(
                    "received a text message - probably a response to some previous query! - {text}",
                );
                self.handle_text_message(text)?;
                Ok(None)
            }
            _ => {
                debug!("received websocket message that's neither 'Binary' nor 'Text'. it's going to get ignored");
                Ok(None)
            }
        }
    }

    fn recover_received_plaintexts(
        &self,
        messages: Vec<Message>,
    ) -> Result<Vec<Vec<u8>>, GatewayClientError> {
        let mut plaintexts = Vec::new();
        for ws_msg in messages {
            if let Some(plaintext) = self.recover_received_plaintext(ws_msg)? {
                plaintexts.push(plaintext)
            }
        }
        Ok(plaintexts)
    }

    fn spawn(self, split_stream: SplitStream<WsConn>, task_client: TaskClient) {
        let fut = async move { self.run(split_stream, task_client).await };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(fut);

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(fut);
    }
}

impl PartiallyDelegatedHandle {
    pub(crate) fn split_and_listen_for_mixnet_messages(
        conn: WsConn,
        packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
        client_bandwidth: ClientBandwidth,
        shutdown: TaskClient,
    ) -> Self {
        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        let (notify_sender, notify_receiver) = oneshot::channel();
        let (stream_sender, stream_receiver) = oneshot::channel();

        let ws_fd = ws_fd(&conn);
        let (sink, stream) = conn.split();

        PartiallyDelegatedRouter::new(
            packet_router,
            shared_key,
            client_bandwidth,
            stream_sender,
            notify_receiver,
        )
        .spawn(stream, shutdown);

        PartiallyDelegatedHandle {
            ws_fd,
            sink_half: sink,
            delegated_stream: (stream_receiver, notify_sender),
        }
    }

    pub(crate) fn ws_fd(&self) -> Option<RawFd> {
        self.ws_fd
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
        let (mut stream_receiver, notify) = self.delegated_stream;

        // check if the split stream didn't error out
        let receive_res = stream_receiver
            .try_recv()
            .expect("stream sender was somehow dropped without sending anything!");

        if let Some(res) = receive_res {
            let _res = res?;
            panic!(
                "This should have NEVER happened - returned a stream before receiving notification"
            )
        }

        // this call failing is incredibly unlikely, but not impossible.
        // basically the gateway connection must have failed after executing previous line but
        // before starting execution of this one.
        notify
            .send(())
            .map_err(|_| GatewayClientError::ConnectionAbruptlyClosed)?;

        let stream_results: Result<_, GatewayClientError> = stream_receiver
            .await
            // Address cancellation of the underlying future past the check
            // in receive_res
            .map_err(|_| GatewayClientError::ConnectionAbruptlyClosed)?;
        let stream = stream_results?;
        // the error is thrown when trying to reunite sink and stream that did not originate
        // from the same split which is impossible to happen here
        Ok(self.sink_half.reunite(stream).unwrap())
    }
}

// we can either have the stream itself or an option to re-obtain it
// by notifying the future owning it to finish the execution and awaiting the result
// which should be almost immediate (or an invalid state which should never, ever happen)
#[derive(Debug)]
pub(crate) enum SocketState {
    Available(Box<WsConn>),
    PartiallyDelegated(PartiallyDelegatedHandle),
    NotConnected,
    Invalid,
}

impl SocketState {
    pub(crate) fn is_available(&self) -> bool {
        matches!(self, SocketState::Available(_))
    }

    pub(crate) fn is_partially_delegated(&self) -> bool {
        matches!(self, SocketState::PartiallyDelegated(_))
    }

    pub(crate) fn is_established(&self) -> bool {
        matches!(
            self,
            SocketState::Available(_) | SocketState::PartiallyDelegated(_)
        )
    }
}
