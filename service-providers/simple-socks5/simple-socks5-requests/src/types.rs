use crate::{Error, ErrorKind, Result};
use std::convert::TryFrom;

pub type ConnectionId = u64;
pub type RemoteAddress = String;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum RequestFlag {
    Connect = 0,
    Send = 1,
    Close = 2,
}

impl TryFrom<u8> for RequestFlag {
    type Error = crate::error::Error;

    fn try_from(value: u8) -> crate::error::Result<Self> {
        match value {
            _ if value == (RequestFlag::Connect as u8) => Ok(Self::Connect),
            _ if value == (RequestFlag::Send as u8) => Ok(Self::Send),
            _ if value == (RequestFlag::Close as u8) => Ok(Self::Close),
            _ => todo!("error"),
        }
    }
}

/*
Request:
    Connect: CONN_FLAG || connection_id || address_length || remote_address_bytes  || request_data_content (vec<u8>)
    Send: SEND_FLAG || connection_id || request_data_content (vec<u8>)
    Close: CLOSE_FLAG || connection_id
*/

#[derive(Debug)]
pub enum Request {
    Connect(ConnectionId, RemoteAddress, Vec<u8>),
    Send(ConnectionId, Vec<u8>),
    Close(ConnectionId),
}

impl Request {
    pub fn new_connect() -> Request {
        todo!()
    }

    pub fn new_send() -> Request {
        todo!()
    }

    pub fn new_close() -> Request {
        todo!()
    }

    // TODO: this dsecription is outdated
    /// Deserialize the destination address and port, the request id,
    /// and the request body from bytes. This is the reverse of SocksRequest::serialize.
    ///
    /// Serialized bytes looks like this:
    ///
    /// ------------------------------------------------------------------------
    /// | address_length | remote_address_bytes | connection_id | request_data |
    /// |      2         |    address_length    |     16     |   ...           |
    /// ------------------------------------------------------------------------
    ///
    /// We return the useful deserialized values.
    pub fn try_from_bytes(b: &[u8]) -> Result<Self> {
        // each request needs to at least contain flag and ConnectionId
        if b.is_empty() {
            return Err(Error::new(ErrorKind::InvalidRequest, "no data provided"));
        }

        if b.len() < 9 {
            return Err(Error::new(
                ErrorKind::InvalidRequest,
                "not enough bytes to parse connection id",
            ));
        }

        let connection_id = u64::from_be_bytes([b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8]]);

        match RequestFlag::try_from(b[0])? {
            RequestFlag::Connect => {
                let connect_request_bytes = &b[8..];

                // we need to be able to read at least 2 bytes that specify address length
                if connect_request_bytes.len() < 2 {
                    return Err(Error::new(
                        ErrorKind::InvalidRequest,
                        "address length too short",
                    ));
                }

                let address_length =
                    u16::from_be_bytes([connect_request_bytes[0], connect_request_bytes[1]])
                        as usize;

                let address_start = 2;
                let address_end = address_start + address_length;
                let address_bytes = &connect_request_bytes[address_start..address_end];
                let remote_address = String::from_utf8_lossy(&address_bytes).to_string();

                let request_data = &connect_request_bytes[address_end..];
                Ok(Request::Connect(
                    connection_id,
                    remote_address,
                    request_data.to_vec(),
                ))
            }
            RequestFlag::Send => Ok(Request::Send(connection_id, b[8..].as_ref().to_vec())),
            RequestFlag::Close => Ok(Request::Close(connection_id)),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Request::Connect(conn_id, remote_address, data) => {
                let remote_address_bytes = remote_address.into_bytes();
                let remote_address_bytes_len = remote_address_bytes.len() as u16;
                std::iter::once(RequestFlag::Connect as u8)
                    .chain(conn_id.to_be_bytes().iter().cloned())
                    .chain(remote_address_bytes_len.to_be_bytes().iter().cloned())
                    .chain(remote_address_bytes.into_iter())
                    .chain(data.into_iter())
                    .collect()
            }
            Request::Send(conn_id, data) => std::iter::once(RequestFlag::Send as u8)
                .chain(conn_id.to_be_bytes().iter().cloned())
                .chain(data.into_iter())
                .collect(),
            Request::Close(conn_id) => std::iter::once(RequestFlag::Close as u8)
                .chain(conn_id.to_be_bytes().iter().cloned())
                .collect(),
        }
    }
}

pub enum Response {}

impl Response {
    pub fn try_from_bytes(b: &[u8]) -> Result<Self> {
        todo!()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        todo!()
    }
}

#[cfg(test)]
mod request_deserialization_tests {
    use super::*;

    #[cfg(test)]
    mod new_connection_tests {
        use super::*;

        #[test]
        fn returns_error_when_zero_bytes() {
            let request_bytes = Vec::new();
            let expected = Error::new(ErrorKind::InvalidRequest, "no data provided");

            assert_eq!(
                expected.to_string(),
                Request::try_from_bytes(&request_bytes)
                    .unwrap_err()
                    .to_string()
            );
        }

        // #[test]
        // fn returns_error_when_address_too_short_for_given_address_length() {
        //     let request_bytes = [0, 1].to_vec(); // there should be a 1-byte remote address, but there's nothing
        //     assert_eq!(Request::try_from_bytes(&request_bytes);
        // }

        // #[test]
        // fn returns_error_when_request_id_too_short() {
        //     // there is a 1-byte remote address, followed by only 1 byte of connection_id, which is too short (must be 16 bytes)
        //     let request_bytes = [0, 1, 0, 1].to_vec();
        //     Request::parse_message(&request_bytes);
        // }

        // #[test]
        // fn works_when_request_is_sized_properly_even_without_data() {
        //     // this one has "foo.com" remote address, correct 16 bytes of connection_id, and 0 bytes request data
        //     let request_bytes = [
        //         0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        //         15, 16,
        //     ]
        //     .to_vec();
        //     let (id, remote_address, data) = Controller::parse_message(request_bytes);
        //     assert_eq!("foo.com".to_string(), remote_address);
        //     assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        //     assert_eq!(Vec::<u8>::new(), data);
        // }

        // #[test]
        // fn works_when_request_is_sized_properly_and_has_data() {
        //     // this one has a 1-byte remote address, correct 16 bytes of connection_id, and 3 bytes request data
        //     let request_bytes = [
        //         0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        //         15, 16, 255, 255, 255,
        //     ]
        //     .to_vec();
        //     let (id, remote_address, data) = Controller::parse_message(request_bytes);
        //     assert_eq!("foo.com".to_string(), remote_address);
        //     assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        //     assert_eq!(vec![255, 255, 255], data);
        // }
    }
}
