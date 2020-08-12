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

#[derive(Debug, PartialEq)]
pub enum RequestError {
    AddressLengthTooShort,
    AddressTooShort,
    ConnectionIdTooShort,
    NoData,
    UnknownRequestFlag,
}

impl TryFrom<u8> for RequestFlag {
    type Error = RequestError;

    fn try_from(value: u8) -> Result<RequestFlag, RequestError> {
        match value {
            _ if value == (RequestFlag::Connect as u8) => Ok(Self::Connect),
            _ if value == (RequestFlag::Send as u8) => Ok(Self::Send),
            _ if value == (RequestFlag::Close as u8) => Ok(Self::Close),
            _ => Err(RequestError::UnknownRequestFlag),
        }
    }
}

/// A request from a SOCKS5 client that a Nym Socks5 service provider should
/// take an action for an application using a (probably local) Nym Socks5 proxy.
#[derive(Debug)]
pub enum Request {
    /// Start a new TCP connection to the specified `RemoteAddress` and send
    /// the request data up the connection.
    Connect(ConnectionId, RemoteAddress, Vec<u8>),

    /// Re-use an existing TCP connection, sending more request data up it.
    Send(ConnectionId, Vec<u8>),

    /// Close an existing TCP connection.
    Close(ConnectionId),
}

impl Request {
    /// Construct a new Request::Connect instance
    pub fn new_connect(
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        data: Vec<u8>,
    ) -> Request {
        Request::Connect(conn_id, remote_addr, data)
    }

    /// Construct a new Request::Send instance
    pub fn new_send(conn_id: ConnectionId, data: Vec<u8>) -> Request {
        Request::Send(conn_id, data)
    }

    /// Construct a new Request::Close instance
    pub fn new_close(conn_id: ConnectionId) -> Request {
        Request::Close(conn_id)
    }

    /// Deserialize the request type, connection id, destination address and port,
    /// and the request body from bytes.
    ///
    /// Serialized bytes looks like this:
    ///
    /// --------------------------------------------------------------------------------------
    ///  request_flag | connection_id | address_length | remote_address_bytes | request_data |
    ///        1      |       8       |      2         |    address_length    |    ...       |
    /// --------------------------------------------------------------------------------------
    ///
    /// The request_flag tells us whether this is a new connection request (`new_connect`),
    /// an already-established connection we should send up (`new_send`), or
    /// a request to close an established connection (`new_close`).
    pub fn try_from_bytes(b: &[u8]) -> Result<Request, RequestError> {
        // each request needs to at least contain flag and ConnectionId
        if b.is_empty() {
            return Err(RequestError::NoData);
        }

        if b.len() < 9 {
            return Err(RequestError::ConnectionIdTooShort);
        }
        let connection_id = u64::from_be_bytes([b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8]]);
        match RequestFlag::try_from(b[0])? {
            RequestFlag::Connect => {
                let connect_request_bytes = &b[9..];

                // we need to be able to read at least 2 bytes that specify address length
                if connect_request_bytes.len() < 2 {
                    return Err(RequestError::AddressLengthTooShort);
                }

                let address_length =
                    u16::from_be_bytes([connect_request_bytes[0], connect_request_bytes[1]])
                        as usize;

                if connect_request_bytes.len() < 2 + address_length {
                    return Err(RequestError::AddressTooShort);
                }

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
            RequestFlag::Send => Ok(Request::Send(connection_id, b[9..].as_ref().to_vec())),
            RequestFlag::Close => Ok(Request::Close(connection_id)),
        }
    }

    /// Serialize a Socks5 request into bytes, so that it can be packed inside
    /// a Sphinx packet, sent through the mixnet, and deserialized by the Socks5
    /// service provider which will make the request.
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

#[cfg(test)]
mod request_deserialization_tests {
    use super::*;

    mod all_request_types {
        use super::*;

        #[test]
        fn returns_error_when_zero_bytes() {
            let request_bytes = Vec::new();
            assert_eq!(
                RequestError::NoData,
                Request::try_from_bytes(&request_bytes).unwrap_err()
            );
        }

        #[test]
        fn returns_error_when_connection_id_too_short() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7].to_vec(); // 7 bytes connection id
            assert_eq!(
                RequestError::ConnectionIdTooShort,
                Request::try_from_bytes(&request_bytes).unwrap_err()
            );
        }
    }

    #[cfg(test)]
    mod sending_data_over_a_new_connection {
        use super::*;

        #[test]
        fn returns_error_when_address_length_is_too_short() {
            let request_bytes1 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8].to_vec(); // 8 bytes connection id, 0 bytes address length (2 were expected)
            let request_bytes2 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0].to_vec(); // 8 bytes connection id, 1 bytes address length (2 were expected)

            assert_eq!(
                RequestError::AddressLengthTooShort,
                Request::try_from_bytes(&request_bytes1).unwrap_err()
            );

            assert_eq!(
                RequestError::AddressLengthTooShort,
                Request::try_from_bytes(&request_bytes2).unwrap_err()
            );
        }

        #[test]
        fn returns_error_when_address_too_short_for_given_address_length() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 1].to_vec(); // 8 bytes connection id, 2 bytes address length, missing address
            assert_eq!(
                RequestError::AddressTooShort,
                Request::try_from_bytes(&request_bytes).unwrap_err()
            );
        }

        #[test]
        fn works_when_request_is_sized_properly_even_without_data() {
            // this one has "foo.com" remote address, correct 8 bytes of connection_id, and 0 bytes request data
            let request_bytes = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
            ]
            .to_vec();
            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Connect(conn_id, remote_address, data) => {
                    assert_eq!("foo.com".to_string(), remote_address);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(Vec::<u8>::new(), data);
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // this one has a 1-byte remote address, correct 16 bytes of connection_id, and 3 bytes request data
            let request_bytes = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
                255,
                255,
                255,
            ]
            .to_vec();

            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Connect(conn_id, remote_address, data) => {
                    assert_eq!("foo.com".to_string(), remote_address);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(vec![255, 255, 255], data);
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod sending_additional_data_over_an_existing_connection {
        use super::*;

        #[test]
        fn works_when_request_is_sized_properly_even_without_data() {
            // correct 8 bytes of connection_id, and 0 bytes request data
            let request_bytes = [RequestFlag::Send as u8, 1, 2, 3, 4, 5, 6, 7, 8].to_vec();
            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Send(conn_id, data) => {
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(Vec::<u8>::new(), data);
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // correct 8 bytes of connection_id, and 3 bytes request data (all 255)
            let request_bytes = [
                RequestFlag::Send as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                255,
                255,
                255,
            ]
            .to_vec();

            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Send(conn_id, data) => {
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(vec![255, 255, 255], data);
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod connection_close_requests {
        use super::*;

        #[test]
        fn can_be_constructed() {
            let request_bytes = [RequestFlag::Close as u8, 1, 2, 3, 4, 5, 6, 7, 8].to_vec();
            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Close(conn_id) => {
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                }
                _ => unreachable!(),
            }
        }
    }
}
