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

use crate::connection_manager::reconnector::ConnectionReconnector;
use crate::connection_manager::writer::ConnectionWriter;
use futures::channel::{mpsc, oneshot};
use futures::future::{abortable, AbortHandle};
use futures::task::Poll;
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::framing::packet::FramedSphinxPacket;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

mod reconnector;
mod writer;

pub(crate) type ResponseSender = Option<oneshot::Sender<io::Result<()>>>;

pub(crate) type ConnectionManagerSender =
    mpsc::UnboundedSender<(FramedSphinxPacket, ResponseSender)>;
type ConnectionManagerReceiver = mpsc::UnboundedReceiver<(FramedSphinxPacket, ResponseSender)>;

enum ConnectionState<'a> {
    Writing(ConnectionWriter),
    Reconnecting(ConnectionReconnector<'a>),
}

pub(crate) struct ConnectionManager<'a> {
    conn_tx: ConnectionManagerSender,
    conn_rx: ConnectionManagerReceiver,

    address: SocketAddr,

    maximum_reconnection_backoff: Duration,
    reconnection_backoff: Duration,
    maximum_reconnection_attempts: u32,

    state: ConnectionState<'a>,
}

impl<'a> Drop for ConnectionManager<'a> {
    fn drop(&mut self) {
        debug!("Connection manager to {:?} is being dropped", self.address)
    }
}

impl<'a> ConnectionManager<'static> {
    pub(crate) async fn new(
        address: SocketAddr,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        connection_timeout: Duration,
        maximum_reconnection_attempts: u32,
    ) -> ConnectionManager<'a> {
        let (conn_tx, conn_rx) = mpsc::unbounded();

        // the blocking call here is fine as initially we want to wait the timeout interval (at most) anyway:
        let tcp_stream_res = std::net::TcpStream::connect_timeout(&address, connection_timeout);

        let initial_state = match tcp_stream_res {
            Ok(stream) => {
                let tokio_stream = tokio::net::TcpStream::from_std(stream).unwrap();
                debug!("managed to establish initial connection to {}", address);
                ConnectionState::Writing(ConnectionWriter::new(tokio_stream))
            }
            Err(e) => {
                warn!("failed to establish initial connection to {} within {:?} ({}). Going into reconnection mode", address, connection_timeout, e);
                ConnectionState::Reconnecting(ConnectionReconnector::new(
                    address,
                    reconnection_backoff,
                    maximum_reconnection_backoff,
                    maximum_reconnection_attempts,
                ))
            }
        };

        ConnectionManager {
            conn_tx,
            conn_rx,
            address,
            maximum_reconnection_backoff,
            reconnection_backoff,
            maximum_reconnection_attempts,
            state: initial_state,
        }
    }

    async fn run(mut self) {
        while let Some(msg) = self.conn_rx.next().await {
            let (framed_packet, res_ch) = msg;

            match self.handle_new_packet(framed_packet).await {
                None => {
                    warn!(
                        "We reached maximum number of attempts trying to reconnect to {}",
                        self.address
                    );
                    return;
                }
                Some(res) => {
                    if let Some(res_ch) = res_ch {
                        if let Err(e) = res_ch.send(res) {
                            error!(
                                "failed to send response on the channel to the caller! - {:?}",
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    /// consumes Self and returns channel for communication as well as an `AbortHandle`
    pub(crate) fn spawn_abortable(self) -> (ConnectionManagerSender, AbortHandle) {
        let sender_clone = self.conn_tx.clone();
        let (abort_fut, abort_handle) = abortable(self.run());

        tokio::spawn(async move { abort_fut.await });

        (sender_clone, abort_handle)
    }

    // Possible future TODO: `Framed<...>` is both a Sink and a Stream,
    // so it is possible to read any responses we might receive (it is also duplex, so that could be
    // done while writing packets themselves). But it'd require slight additions to `SphinxCodec`
    async fn handle_new_packet(&mut self, packet: FramedSphinxPacket) -> Option<io::Result<()>> {
        // we don't do a match here as it's possible to transition from ConnectionState::Reconnecting to ConnectionState::Writing
        // in this function call. And if that happens, we want to send the packet we have received.
        if let ConnectionState::Reconnecting(conn_reconnector) = &mut self.state {
            // do a single poll rather than await for future to completely resolve
            let new_connection = match futures::poll(conn_reconnector).await {
                Poll::Pending => {
                    debug!("The packet is getting dropped - there's nowhere to send it");
                    return Some(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "connection is broken - reconnection is in progress",
                    )));
                }
                Poll::Ready(conn) => conn,
            };

            match new_connection {
                Ok(new_conn) => {
                    debug!("Managed to reconnect to {}!", self.address);
                    self.state = ConnectionState::Writing(ConnectionWriter::new(new_conn));
                }
                Err(_) => return None,
            }
        }

        // we must be in writing state if we are here, either by being here from beginning or just
        // transitioning from reconnecting
        if let ConnectionState::Writing(conn_writer) = &mut self.state {
            return if let Err(e) = conn_writer.send(packet).await {
                warn!(
                    "Failed to forward message - {:?}. Starting reconnection procedure...",
                    e
                );
                self.state = ConnectionState::Reconnecting(ConnectionReconnector::new(
                    self.address,
                    self.reconnection_backoff,
                    self.maximum_reconnection_backoff,
                    self.maximum_reconnection_attempts,
                ));
                Some(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "connection is broken - reconnection is in progress",
                )))
            } else {
                Some(Ok(()))
            };
        }

        unreachable!();
    }
}
