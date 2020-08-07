use crate::connection::Connection;
use simple_socks5_requests::{ConnectionId, RemoteAddress, Request, Response};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ConnectionError;

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
                self.close_connection(conn_id)?;
                Ok(None)
            }
        }
    }

    async fn create_new_connection(
        &mut self,
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        init_data: Vec<u8>,
    ) -> Result<Response, ConnectionError> {
        let mut connection = Connection::new(conn_id, remote_addr, &init_data)
            .await
            .map_err(|err| log::error!("new connection failed with: {}", err))
            .unwrap();

        let response_data = connection
            .try_read_response_data()
            .await
            .map_err(|err| log::error!("error reading response data: {}", err))
            .unwrap();

        self.open_connections.insert(conn_id, connection);
        Ok(Response::new(conn_id, response_data))
    }

    async fn send_to_connection(
        &mut self,
        conn_id: ConnectionId,
        data: Vec<u8>,
    ) -> Result<Response, ConnectionError> {
        let connection = self
            .open_connections
            .get_mut(&conn_id)
            .expect("TODO: dont panic - connection doesn't exist");
        connection.send_data(&data).await.expect("todo: error");

        let response_data = connection
            .try_read_response_data()
            .await
            .expect("todo: error handling");
        Ok(Response::new(conn_id, response_data))
    }

    fn close_connection(&mut self, conn_id: ConnectionId) -> Result<(), ConnectionError> {
        match self.open_connections.remove(&conn_id) {
            // I *think* connection is closed implicitly on drop, but I'm not 100% sure!
            Some(_conn) => (),
            // TODO: don't panic
            None => panic!("tried to close non-existing connection! - {}", conn_id),
        }

        Ok(())
    }
}
