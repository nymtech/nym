use crate::ConnectionId;
use crate::{Error, ErrorKind, Result};

/// A remote network response retrieved by the Socks5 service provider. This
/// can be serialized and sent back through the mixnet to the requesting
/// application.
#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
    pub connection_id: ConnectionId,
}

impl Response {
    /// Constructor for responses
    pub fn new(connection_id: ConnectionId, data: Vec<u8>) -> Self {
        Response {
            connection_id,
            data,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Response> {
        if b.len() < 8 {
            return Err(Error::new(
                ErrorKind::InvalidResponse,
                "response bytes too short",
            ));
        }

        let mut connection_id_bytes = b.to_vec();
        let data = connection_id_bytes.split_off(8);

        let connection_id = u64::from_be_bytes([
            connection_id_bytes[0],
            connection_id_bytes[1],
            connection_id_bytes[2],
            connection_id_bytes[3],
            connection_id_bytes[4],
            connection_id_bytes[5],
            connection_id_bytes[6],
            connection_id_bytes[7],
        ]);

        let response = Response::new(connection_id, data);
        Ok(response)
    }

    /// Serializes the response into bytes so that it can be sent back through
    /// the mixnet to the requesting application.
    pub fn into_bytes(self) -> Vec<u8> {
        self.connection_id
            .to_be_bytes()
            .iter()
            .cloned()
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
        let expected = Error::new(ErrorKind::InvalidResponse, "response bytes too short");

        assert_eq!(
            expected.to_string(),
            Response::try_from_bytes(&response_bytes)
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    fn fails_when_connection_id_bytes_are_too_short() {
        let response_bytes = vec![0, 1, 2, 3, 4, 5, 6];
        let expected = Error::new(ErrorKind::InvalidResponse, "response bytes too short");

        assert_eq!(
            expected.to_string(),
            Response::try_from_bytes(&response_bytes)
                .unwrap_err()
                .to_string()
        );
    }

    #[test]
    fn works_even_with_no_data() {
        let response_bytes = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let expected = Response::new(u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]), Vec::new());
        let actual = Response::try_from_bytes(&response_bytes).unwrap();
        assert_eq!(expected.connection_id, actual.connection_id);
        assert_eq!(expected.data, actual.data);
    }

    #[test]
    fn works_when_there_is_data() {
        let response_bytes = vec![0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255];
        let expected = Response::new(
            u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
            vec![255, 255, 255],
        );
        let actual = Response::try_from_bytes(&response_bytes).unwrap();
        assert_eq!(expected.connection_id, actual.connection_id);
        assert_eq!(expected.data, actual.data);
    }
}
