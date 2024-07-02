// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayClientError;
use crate::packet_router::PacketRouter;
use crate::traits::GatewayPacketRouter;
use crate::{cleanup_socket_messages, try_decrypt_binary_message};
use futures::channel::oneshot;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::*;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_gateway_requests::ServerResponse;
use nym_task::TaskClient;
use si_scale::helpers::bibytes2;
use std::os::raw::c_int as RawFd;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tungstenite::Message;

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

pub(crate) fn ws_fd(_conn: &WsConn) -> Option<RawFd> {
    #[cfg(unix)]
    match _conn.get_ref() {
        MaybeTlsStream::Plain(stream) => Some(stream.as_raw_fd()),
        &_ => None,
    }
    #[cfg(not(unix))]
    None
}

// disgusting? absolutely, but does the trick for now
static LAST_LOGGED_BANDWIDTH_TS: AtomicI64 = AtomicI64::new(0);

fn maybe_log_bandwidth(remaining: i64) {
    // SAFETY: this value is always populated with valid timestamps
    let last =
        OffsetDateTime::from_unix_timestamp(LAST_LOGGED_BANDWIDTH_TS.load(Ordering::Relaxed))
            .unwrap();
    let now = OffsetDateTime::now_utc();
    if last + Duration::from_secs(10) < now {
        log::info!("remaining bandwidth: {}", bibytes2(remaining as f64));
        LAST_LOGGED_BANDWIDTH_TS.store(now.unix_timestamp(), Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub(crate) struct PartiallyDelegated {
    sink_half: SplitSink<WsConn, Message>,
    delegated_stream: (SplitStreamReceiver, oneshot::Sender<()>),
    ws_fd: Option<RawFd>,
}

impl PartiallyDelegated {
    fn recover_received_plaintexts(
        ws_msgs: Vec<Message>,
        shared_key: &SharedKeys,
        bandwidth_remaining: Arc<AtomicI64>,
    ) -> Result<Vec<Vec<u8>>, GatewayClientError> {
        let mut plaintexts = Vec::with_capacity(ws_msgs.len());
        for ws_msg in ws_msgs {
            match ws_msg {
                Message::Binary(bin_msg) => {
                    // this function decrypts the request and checks the MAC
                    if let Some(plaintext) = try_decrypt_binary_message(bin_msg, shared_key) {
                        plaintexts.push(plaintext)
                    }
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
                    match ServerResponse::try_from(text)
                        .map_err(|_| GatewayClientError::MalformedResponse)?
                    {
                        ServerResponse::Send {
                            remaining_bandwidth,
                        } => {
                            maybe_log_bandwidth(remaining_bandwidth);
                            bandwidth_remaining
                                .store(remaining_bandwidth, std::sync::atomic::Ordering::Release)
                        }
                        ServerResponse::Error { message } => {
                            error!("gateway failure: {message}");
                            return Err(GatewayClientError::GatewayError(message));
                        }
                        other => {
                            warn!(
                                "received illegal message of type {} in an authenticated client",
                                other.name()
                            )
                        }
                    }

                    continue;
                }
                _ => continue,
            }
        }
        Ok(plaintexts)
    }

    fn route_socket_messages(
        ws_msgs: Vec<Message>,
        packet_router: &PacketRouter,
        shared_key: &SharedKeys,
        bandwidth_remaining: Arc<AtomicI64>,
    ) -> Result<(), GatewayClientError> {
        let plaintexts =
            Self::recover_received_plaintexts(ws_msgs, shared_key, bandwidth_remaining)?;
        packet_router.route_received(plaintexts)
    }

    pub(crate) fn split_and_listen_for_mixnet_messages(
        conn: WsConn,
        mut packet_router: PacketRouter,
        shared_key: Arc<SharedKeys>,
        bandwidth_remaining: Arc<AtomicI64>,
        mut shutdown: TaskClient,
    ) -> Self {
        // when called for, it NEEDS TO yield back the stream so that we could merge it and
        // read control request responses.
        let (notify_sender, notify_receiver) = oneshot::channel();
        let (stream_sender, stream_receiver) = oneshot::channel();

        let ws_fd = ws_fd(&conn);

        let (sink, mut stream) = conn.split();

        let mixnet_receiver_future = async move {
            let mut notify_receiver = notify_receiver;
            let mut chunk_stream = (&mut stream).ready_chunks(8);

            let ret_err = loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        log::trace!("GatewayClient listener: Received shutdown");
                        log::debug!("GatewayClient listener: Exiting");
                        return;
                    }
                    _ = &mut notify_receiver => {
                        break Ok(());
                    }
                    msgs = chunk_stream.next() => {
                        let ws_msgs = match cleanup_socket_messages(msgs) {
                            Err(err) => break Err(err),
                            Ok(msgs) => msgs
                        };

                        if let Err(err) = Self::route_socket_messages(ws_msgs, &packet_router, shared_key.as_ref(), bandwidth_remaining.clone()) {
                            log::error!("Route socket messages failed: {err}");
                            break Err(err)
                        }
                    }
                };
            };

            if match ret_err {
                Err(err) => stream_sender.send(Err(err)),
                Ok(_) => {
                    packet_router.mark_as_success();
                    shutdown.mark_as_success();
                    stream_sender.send(Ok(stream))
                }
            }
            .is_err()
            {
                warn!("failed to send back `mixnet_receiver_future` result on the oneshot channel")
            }
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(mixnet_receiver_future);

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(mixnet_receiver_future);

        PartiallyDelegated {
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
    PartiallyDelegated(PartiallyDelegated),
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
