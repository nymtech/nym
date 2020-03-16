use crate::connection_manager::reconnector::ConnectionReconnector;
use crate::connection_manager::writer::ConnectionWriter;
use futures::channel::{mpsc, oneshot};
use futures::task::Poll;
use futures::{AsyncWriteExt, StreamExt};
use log::*;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Handle;

mod reconnector;
mod writer;

pub(crate) type ResponseSender = Option<oneshot::Sender<io::Result<()>>>;

pub(crate) type ConnectionManagerSender = mpsc::UnboundedSender<(Vec<u8>, ResponseSender)>;
type ConnectionManagerReceiver = mpsc::UnboundedReceiver<(Vec<u8>, ResponseSender)>;

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
}

impl<'a> ConnectionManager<'static> {
    pub(crate) async fn new(
        address: SocketAddr,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
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
                let (msg_content, res_ch) = msg;
                let res = self.handle_new_message(msg_content).await;
                if let Some(res_ch) = res_ch {
                    if let Err(e) = res_ch.send(res) {
                        error!(
                            "failed to send response on the channel to the caller! - {:?}",
                            e
                        );
                    }
                }
            }
        });
        sender_clone
    }

    async fn handle_new_message(&mut self, msg: Vec<u8>) -> io::Result<()> {
        if let ConnectionState::Reconnecting(conn_reconnector) = &mut self.state {
            // do a single poll rather than await for future to completely resolve
            let new_connection = match futures::poll!(conn_reconnector) {
                Poll::Pending => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "connection is broken - reconnection is in progress",
                    ))
                }
                Poll::Ready(conn) => conn,
            };

            debug!("Managed to reconnect to {}!", self.address);
            self.state = ConnectionState::Writing(ConnectionWriter::new(new_connection));
        }

        // we must be in writing state if we are here, either by being here from beginning or just
        // transitioning from reconnecting
        if let ConnectionState::Writing(conn_writer) = &mut self.state {
            return match conn_writer.write_all(msg.as_ref()).await {
                // if we failed to write to connection we should reconnect
                // TODO: is this true? can we fail to write to a connection while it still remains open and valid?

                // TODO: change connection writer to somehow also poll for responses and
                // change return type of this method from io::Result<> to io::Result<Vec<u8>>
                Ok(_) => Ok(()),
                Err(e) => {
                    trace!("Creating connection reconnector!");
                    self.state = ConnectionState::Reconnecting(ConnectionReconnector::new(
                        self.address,
                        self.reconnection_backoff,
                        self.maximum_reconnection_backoff,
                    ));
                    Err(e)
                }
            };
        };

        unreachable!();
    }
}
