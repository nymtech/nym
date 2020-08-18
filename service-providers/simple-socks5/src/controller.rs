use crate::connection::Connection;
use futures::lock::Mutex;
use simple_socks5_requests::{ConnectionId, RemoteAddress, Request, Response};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub enum ConnectionError {
    ConnectionFailed(io::Error),
    MissingConnection,
}

impl From<io::Error> for ConnectionError {
    fn from(e: io::Error) -> Self {
        ConnectionError::ConnectionFailed(e)
    }
}

// clone is fine here because HashMap is never cloned, the pointer is
#[derive(Clone)]
pub(crate) struct Controller {
    open_connections: Arc<Mutex<HashMap<ConnectionId, Connection>>>,
}

impl Controller {
    pub(crate) fn new() -> Self {
        Controller {
            open_connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) async fn process_request(
        &mut self,
        request: Request,
    ) -> Result<Option<Response>, ConnectionError> {
        match request {
            Request::Connect(conn_id, remote_addr, data) => {
                let response = self
                    .create_new_connection(conn_id, remote_addr, data)
                    .await?;
                Ok(Some(response))
            }
            Request::Send(conn_id, data) => {
                let response = self.send_to_connection(conn_id, data).await?;
                Ok(Some(response))
            }
            Request::Close(conn_id) => {
                self.close_connection(conn_id).await?;
                Ok(None)
            }
        }
    }

    async fn insert_connection(
        &mut self,
        conn_id: ConnectionId,
        conn: Connection,
    ) -> Option<Connection> {
        self.open_connections.lock().await.insert(conn_id, conn)
    }

    async fn create_new_connection(
        &mut self,
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        init_data: Vec<u8>,
    ) -> Result<Response, ConnectionError> {
        println!("Connecting {} to remote {}", conn_id, remote_addr);
        let mut connection = Connection::new(conn_id, remote_addr, &init_data).await?;
        let response_data = connection.try_read_response_data().await?;
        self.insert_connection(conn_id, connection).await;
        Ok(Response::new(conn_id, response_data))
    }

    async fn send_to_connection(
        &mut self,
        conn_id: ConnectionId,
        data: Vec<u8>,
    ) -> Result<Response, ConnectionError> {
        let mut open_connections_guard = self.open_connections.lock().await;

        // TODO: is it possible to do it more nicely than getting lock -> removing connection ->
        // processing -> reacquiring lock and putting connection back?

        let mut connection = open_connections_guard
            .remove(&conn_id)
            .ok_or_else(|| ConnectionError::MissingConnection)?;
        connection.send_data(&data).await?;

        drop(open_connections_guard);
        let response_data = connection.try_read_response_data().await?;
        self.insert_connection(conn_id, connection).await;

        Ok(Response::new(conn_id, response_data))
    }

    async fn close_connection(&mut self, conn_id: ConnectionId) -> Result<(), ConnectionError> {
        match self.open_connections.lock().await.remove(&conn_id) {
            // I *think* connection is closed implicitly on drop, but I'm not 100% sure!
            Some(_conn) => (),
            None => log::error!("tried to close non-existent connection - {}", conn_id),
        }

        Ok(())
    }
}
