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
use nymsphinx::SphinxPacket;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

mod reconnector;
mod writer;

pub(crate) type ResponseSender = Option<oneshot::Sender<io::Result<()>>>;

pub(crate) type ConnectionManagerSender = mpsc::UnboundedSender<(SphinxPacket, ResponseSender)>;
type ConnectionManagerReceiver = mpsc::UnboundedReceiver<(SphinxPacket, ResponseSender)>;

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

    state: ConnectionState<'a>,
    pending_messages_buffer: Vec<SphinxPacket>,
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
                ))
            }
        };

        ConnectionManager {
            conn_tx,
            conn_rx,
            address,
            maximum_reconnection_backoff,
            reconnection_backoff,
            state: initial_state,
            pending_messages_buffer: Vec::new(),
        }
    }

    async fn run(mut self) {
        while let Some(msg) = self.conn_rx.next().await {
            let (msg_content, res_ch) = msg;
            let res = self.handle_new_packet(msg_content).await;
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

    /// consumes Self and returns channel for communication as well as an `AbortHandle`
    pub(crate) fn start_abortable(self, handle: &Handle) -> (ConnectionManagerSender, AbortHandle) {
        let sender_clone = self.conn_tx.clone();
        let (abort_fut, abort_handle) = abortable(self.run());

        handle.spawn(async move { abort_fut.await });

        (sender_clone, abort_handle)
    }

    // Possible future TODO: `Framed<...>` is both a Sink and a Stream,
    // so it is possible to read any responses we might receive (it is also duplex, so that could be
    // done while writing packets themselves). But it'd require slight additions to `SphinxCodec`
    async fn handle_new_packet(&mut self, packet: SphinxPacket) -> io::Result<()> {
        if let ConnectionState::Reconnecting(conn_reconnector) = &mut self.state {
            // do a single poll rather than await for future to completely resolve
            let new_connection = match futures::poll(conn_reconnector).await {
                Poll::Pending => {
                    // make sure we don't lose the received packet
                    self.pending_messages_buffer.push(packet);
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "connection is broken - reconnection is in progress",
                    )
                    .into());
                }
                Poll::Ready(conn) => conn,
            };

            debug!("Managed to reconnect to {}!", self.address);
            self.state = ConnectionState::Writing(ConnectionWriter::new(new_connection));
        }

        // we must be in writing state if we are here, either by being here from beginning or just
        // transitioning from reconnecting
        if let ConnectionState::Writing(conn_writer) = &mut self.state {
            // check if we have any pending writes
            return if !self.pending_messages_buffer.is_empty() {
                let pending_messages =
                    std::mem::replace(&mut self.pending_messages_buffer, Vec::new());
                let mut send_stream = futures::stream::iter(
                    pending_messages
                        .into_iter()
                        .chain(std::iter::once(packet))
                        .map(|packet| Ok(packet)),
                );

                match conn_writer.send_all(&mut send_stream).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        // whatever wasn't successfully sent, put it back into the buffer
                        // (Looking at Future impl for SendAll, I *think* it does not consume
                        // items it failed to send)
                        self.pending_messages_buffer = send_stream
                            .filter_map(|x| async move { x.ok() }) // presumably all items will be an 'Ok'?
                            .collect()
                            .await;

                        warn!(
                            "Failed to forward messages - {:?}. Starting reconnection procedure...",
                            e
                        );
                        self.state = ConnectionState::Reconnecting(ConnectionReconnector::new(
                            self.address,
                            self.reconnection_backoff,
                            self.maximum_reconnection_backoff,
                        ));
                        Err(e.into())
                    }
                }
            } else {
                if let Err(e) = conn_writer.send(packet).await {
                    warn!(
                        "Failed to forward message - {:?}. Starting reconnection procedure...",
                        e
                    );
                    self.state = ConnectionState::Reconnecting(ConnectionReconnector::new(
                        self.address,
                        self.reconnection_backoff,
                        self.maximum_reconnection_backoff,
                    ));
                }
                Ok(())
            };
        }

        unreachable!();
    }
}
