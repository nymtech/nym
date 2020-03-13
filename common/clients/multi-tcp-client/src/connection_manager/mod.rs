use crate::connection_manager::reconnector::ConnectionReconnector;
use crate::connection_manager::writer::ConnectionWriter;
use crate::error_reader::ConnectionErrorSender;
use futures::channel::mpsc;
use futures::task::Poll;
use futures::{AsyncWriteExt, StreamExt};
use log::*;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

mod reconnector;
mod writer;

pub(crate) type ConnectionManagerSender = mpsc::UnboundedSender<Vec<u8>>;
type ConnectionManagerReceiver = mpsc::UnboundedReceiver<Vec<u8>>;

enum ConnectionState<'a> {
    Writing(ConnectionWriter),
    Reconnecting(ConnectionReconnector<'a>),
}

pub(crate) struct ConnectionManager<'a> {
    conn_tx: ConnectionManagerSender,
    conn_rx: ConnectionManagerReceiver,

    errors_tx: ConnectionErrorSender,
    address: SocketAddr,

    maximum_reconnection_backoff: Duration,
    reconnection_backoff: Duration,

    state: ConnectionState<'a>,
}

impl<'a> ConnectionManager<'static> {
    pub(crate) async fn new(
        address: SocketAddr,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        errors_tx: ConnectionErrorSender,
    ) -> ConnectionManager<'a> {
        let (conn_tx, conn_rx) = mpsc::unbounded();

        // based on initial connection we will either have a writer or a reconnector
        let state = match tokio::net::TcpStream::connect(address).await {
            Ok(conn) => ConnectionState::Writing(ConnectionWriter::new(conn)),
            Err(e) => {
                warn!(
                    "failed to establish initial connection to {} ({}). Going into reconnection mode",
                    address, e
                );
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
            errors_tx,
            address,
            maximum_reconnection_backoff,
            reconnection_backoff,
            state,
        }
    }

    /// consumes Self and returns channel for communication
    pub(crate) fn start(mut self, handle: &Handle) -> ConnectionManagerSender {
        let sender_clone = self.conn_tx.clone();
        handle.spawn(async move {
            while let Some(msg) = self.conn_rx.next().await {
                self.handle_new_message(msg).await;
            }
        });
        sender_clone
    }

    async fn handle_new_message(&mut self, msg: Vec<u8>) {
        if let ConnectionState::Reconnecting(conn_reconnector) = &mut self.state {
            // do a single poll rather than await for future to completely resolve
            let new_connection = match futures::poll!(conn_reconnector) {
                Poll::Pending => {
                    self.errors_tx
                        .unbounded_send((
                            self.address,
                            Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "connection is broken - reconnection is in progress",
                            )),
                        ))
                        .unwrap();
                    return;
                }
                Poll::Ready(conn) => conn,
            };

            debug!("Managed to reconnect to {}!", self.address);
            self.state = ConnectionState::Writing(ConnectionWriter::new(new_connection));
        }

        // we must be in writing state if we are here, either by being here from beginning or just
        // transitioning from reconnecting
        if let ConnectionState::Writing(conn_writer) = &mut self.state {
            if let Err(e) = conn_writer.write_all(msg.as_ref()).await {
                debug!("Creating connection reconnector!");
                self.state = ConnectionState::Reconnecting(ConnectionReconnector::new(
                    self.address,
                    self.reconnection_backoff,
                    self.maximum_reconnection_backoff,
                ));
                self.errors_tx
                    .unbounded_send((self.address, Err(e)))
                    .unwrap();
            }
        };
    }
}
