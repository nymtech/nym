use crate::proxy::connection;
use connection::Connection;
use simple_socks5_requests::{ConnectionId, RemoteAddress, Request};
use std::collections::HashMap;

pub struct TodoError;

pub(crate) struct Controller {
    // TODO: I've got a feeling this will need to have a mutex slapped on it, but we'll see
    open_connections: HashMap<ConnectionId, Connection>,
}

impl Controller {
    pub(crate) fn new() -> Self {
        Controller {
            open_connections: HashMap::new(),
        }
    }

    pub(crate) async fn process_request(&mut self, request: Request) -> Result<(), TodoError> {
        match request {
            Request::Connect(conn_id, remote_addr, data) => {
                self.create_new_connection(conn_id, remote_addr, data).await
            }
            Request::Send(conn_id, data) => self.send_to_connection(conn_id, data).await,
            Request::Close(conn_id) => self.close_connection(conn_id),
        }
    }

    async fn create_new_connection(
        &mut self,
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        init_data: Vec<u8>,
    ) -> Result<(), TodoError> {
        Connection::new(conn_id, remote_addr, &init_data)
            .await
            .expect("todo: error handling");
        Ok(())
    }

    async fn send_to_connection(
        &mut self,
        conn_id: ConnectionId,
        data: Vec<u8>,
    ) -> Result<(), TodoError> {
        let connection = self
            .open_connections
            .get_mut(&conn_id)
            .expect("TODO: dont panic - connection doesn't exist");
        connection.send_data(&data).await.expect("todo: error");
        Ok(())
    }

    fn close_connection(&mut self, conn_id: ConnectionId) -> Result<(), TodoError> {
        match self.open_connections.remove(&conn_id) {
            // I *think* connection is closed implicitly on drop, but I'm not 100% sure!
            Some(_conn) => (),
            // TODO: don't panic
            None => panic!("tried to close non-existing connection! - {}", conn_id),
        }

        Ok(())
    }
}
