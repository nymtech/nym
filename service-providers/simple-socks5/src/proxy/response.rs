use crate::proxy::connection;

pub struct Response {
    data: Vec<u8>,
    connection_id: connection::Id,
}

impl Response {
    pub fn new(connection_id: [u8; 16], data: Vec<u8>) -> Response {
        Response {
            data,
            connection_id,
        }
    }

    /// Serializes the response as `connection_id || data`, returning a byte vector
    pub fn serialize(self) -> Vec<u8> {
        self.connection_id
            .iter()
            .cloned()
            .chain(self.data.into_iter())
            .collect()
    }
}
