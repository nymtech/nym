use crate::connection_manager::reconnector::ConnectionReconnector;
use crate::connection_manager::writer::ConnectionWriter;
use futures::task::Poll;
use futures::AsyncWriteExt;
use log::*;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

mod reconnector;
mod writer;

enum ConnectionState {
    Writing(ConnectionWriter),
    Reconnecting(ConnectionReconnector),
}

pub(crate) struct ConnectionManager {
    address: SocketAddr,

    maximum_reconnection_backoff: Duration,
    reconnection_backoff: Duration,

    state: ConnectionState,
}

impl ConnectionManager {
    pub(crate) async fn new(
        address: SocketAddr,
        reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
    ) -> Self {
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
            address,
            maximum_reconnection_backoff,
            reconnection_backoff,
            state,
        }
    }

    pub(crate) async fn send(&mut self, msg: &[u8]) -> io::Result<()> {
        if let ConnectionState::Reconnecting(conn_reconnector) = &mut self.state {
            // do a single poll rather than await for future to completely resolve
            // TODO: if we call poll ourselves here, will the Waker still call it itself later on?
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
            return match conn_writer.write_all(msg).await {
                // if we failed to write to connection we should reconnect
                // TODO: is this true? can we fail to write to a connection while it still remains open and valid?
                Ok(res) => Ok(res),
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
        unreachable!()
    }
}
