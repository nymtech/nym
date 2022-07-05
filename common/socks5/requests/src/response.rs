use crate::ConnectionId;

#[derive(Debug, PartialEq, Eq)]
pub enum ResponseError {
    ConnectionIdTooShort,
    NoData,
}
/// A remote network response retrieved by the Socks5 service provider. This
/// can be serialized and sent back through the mixnet to the requesting
/// application.
#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
    pub connection_id: ConnectionId,
    pub is_closed: bool,
}

impl Response {
    /// Constructor for responses
    pub fn new(connection_id: ConnectionId, data: Vec<u8>, is_closed: bool) -> Self {
        Response {
            data,
            connection_id,
            is_closed,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Response, ResponseError> {
        if b.is_empty() {
            return Err(ResponseError::NoData);
        }

        let is_closed = b[0] != 0;

        if b.len() < 9 {
            return Err(ResponseError::ConnectionIdTooShort);
        }

        let mut connection_id_bytes = b.to_vec();
        let data = connection_id_bytes.split_off(9);

        let connection_id = u64::from_be_bytes([
            connection_id_bytes[1],
            connection_id_bytes[2],
            connection_id_bytes[3],
            connection_id_bytes[4],
            connection_id_bytes[5],
            connection_id_bytes[6],
            connection_id_bytes[7],
            connection_id_bytes[8],
        ]);

        let response = Response::new(connection_id, data, is_closed);
        Ok(response)
    }

    /// Serializes the response into bytes so that it can be sent back through
    /// the mixnet to the requesting application.
    pub fn into_bytes(self) -> Vec<u8> {
        std::iter::once(self.is_closed as u8)
            .chain(self.connection_id.to_be_bytes().iter().cloned())
            .chain(self.data.into_iter())
            .collect()
    }
}

#[cfg(test)]
mod constructing_socks5_responses_from_bytes {
    use super::*;

    #[test]
    fn fails_when_zero_bytes_are_supplied() {
        let response_bytes = Vec::new();

        assert_eq!(
            ResponseError::NoData,
            Response::try_from_bytes(&response_bytes).unwrap_err()
        );
    }

    #[test]
    fn fails_when_connection_id_bytes_are_too_short() {
        let response_bytes = vec![0, 1, 2, 3, 4, 5, 6];
        assert_eq!(
            ResponseError::ConnectionIdTooShort,
            Response::try_from_bytes(&response_bytes).unwrap_err()
        );
    }

    #[test]
    fn works_when_there_is_no_data() {
        let response_bytes = vec![0, 0, 1, 2, 3, 4, 5, 6, 7];
        let expected = Response::new(
            u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
            Vec::new(),
            false,
        );
        let actual = Response::try_from_bytes(&response_bytes).unwrap();
        assert_eq!(expected.connection_id, actual.connection_id);
        assert_eq!(expected.data, actual.data);
        assert_eq!(expected.is_closed, actual.is_closed);
    }

    #[test]
    fn works_when_there_is_data() {
        let response_bytes = vec![0, 0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255];
        let expected = Response::new(
            u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
            vec![255, 255, 255],
            false,
        );
        let actual = Response::try_from_bytes(&response_bytes).unwrap();
        assert_eq!(expected.connection_id, actual.connection_id);
        assert_eq!(expected.data, actual.data);
        assert_eq!(expected.is_closed, actual.is_closed);
    }
}
