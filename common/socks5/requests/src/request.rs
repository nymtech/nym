use nymsphinx_addressing::clients::{Recipient, RecipientFormattingError};
use std::convert::TryFrom;
use std::fmt::{self};

pub type ConnectionId = u64;
pub type RemoteAddress = String;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum RequestFlag {
    Connect = 0,
    Send = 1,
}

#[derive(Debug)]
pub enum RequestError {
    AddressLengthTooShort,
    AddressTooShort,
    ConnectionIdTooShort,
    NoData,
    UnknownRequestFlag,
    ReturnAddressTooShort,
    MalformedReturnAddress(RecipientFormattingError),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::AddressLengthTooShort => {
                write!(f, "not enough bytes to recover the length of the address")
            }
            RequestError::AddressTooShort => write!(f, "not enough bytes to recover the address"),
            RequestError::ConnectionIdTooShort => {
                write!(f, "not enough bytes to recover the connection id")
            }
            RequestError::NoData => write!(f, "no data provided"),
            RequestError::UnknownRequestFlag => write!(f, "request of unknown type"),
            RequestError::ReturnAddressTooShort => write!(f, "too short return address"),
            RequestError::MalformedReturnAddress(recipient_err) => {
                write!(f, "malformed return address - {}", recipient_err)
            }
        }
    }
}

impl std::error::Error for RequestError {}

impl RequestError {
    pub fn is_malformed_return(&self) -> bool {
        matches!(self, RequestError::MalformedReturnAddress(_))
    }
}

impl TryFrom<u8> for RequestFlag {
    type Error = RequestError;

    fn try_from(value: u8) -> Result<RequestFlag, RequestError> {
        match value {
            _ if value == (RequestFlag::Connect as u8) => Ok(Self::Connect),
            _ if value == (RequestFlag::Send as u8) => Ok(Self::Send),
            _ => Err(RequestError::UnknownRequestFlag),
        }
    }
}

#[derive(Debug)]
pub struct ConnectRequest {
    pub conn_id: ConnectionId,
    pub remote_addr: RemoteAddress,
    pub return_address: Recipient,
}

/// A request from a SOCKS5 client that a Nym Socks5 service provider should
/// take an action for an application using a (probably local) Nym Socks5 proxy.
#[derive(Debug)]
pub enum Request {
    /// Start a new TCP connection to the specified `RemoteAddress` and send
    /// the request data up the connection.
    /// All responses produced on this `ConnectionId` should come back to the specified `Recipient`
    Connect(Box<ConnectRequest>),

    /// Re-use an existing TCP connection, sending more request data up it.
    Send(ConnectionId, Vec<u8>, bool),
}

impl Request {
    /// Construct a new Request::Connect instance
    pub fn new_connect(
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        return_address: Recipient,
    ) -> Request {
        Request::Connect(Box::new(ConnectRequest {
            conn_id,
            remote_addr,
            return_address,
        }))
    }

    /// Construct a new Request::Send instance
    pub fn new_send(conn_id: ConnectionId, data: Vec<u8>, local_closed: bool) -> Request {
        Request::Send(conn_id, data, local_closed)
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
                let remote_address = String::from_utf8_lossy(address_bytes).to_string();

                // just a temporary reference to mid-slice for ease of use
                let recipient_data_bytes = &connect_request_bytes[address_end..];

                if recipient_data_bytes.len() != Recipient::LEN {
                    return Err(RequestError::ReturnAddressTooShort);
                }

                let mut return_bytes = [0u8; Recipient::LEN];
                return_bytes.copy_from_slice(&recipient_data_bytes[..Recipient::LEN]);
                let return_address = Recipient::try_from_bytes(return_bytes)
                    .map_err(RequestError::MalformedReturnAddress)?;

                Ok(Request::new_connect(
                    connection_id,
                    remote_address,
                    return_address,
                ))
            }
            RequestFlag::Send => {
                let local_closed = b[9] != 0;
                let data = b[10..].as_ref().to_vec();

                Ok(Request::Send(connection_id, data, local_closed))
            }
        }
    }

    /// Serialize a Socks5 request into bytes, so that it can be packed inside
    /// a Sphinx packet, sent through the mixnet, and deserialized by the Socks5
    /// service provider which will make the request.
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            // connect is: CONN_FLAG || CONN_ID || REMOTE_LEN || REMOTE || RETURN
            Request::Connect(req) => {
                let remote_address_bytes = req.remote_addr.into_bytes();
                let remote_address_bytes_len = remote_address_bytes.len() as u16;

                std::iter::once(RequestFlag::Connect as u8)
                    .chain(req.conn_id.to_be_bytes().iter().cloned())
                    .chain(remote_address_bytes_len.to_be_bytes().iter().cloned())
                    .chain(remote_address_bytes.into_iter())
                    .chain(req.return_address.to_bytes().iter().cloned())
                    .collect()
            }
            Request::Send(conn_id, data, local_closed) => std::iter::once(RequestFlag::Send as u8)
                .chain(conn_id.to_be_bytes().iter().cloned())
                .chain(std::iter::once(local_closed as u8))
                .chain(data.into_iter())
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
            match Request::try_from_bytes(&request_bytes).unwrap_err() {
                RequestError::NoData => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_when_connection_id_too_short() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7].to_vec(); // 7 bytes connection id
            match Request::try_from_bytes(&request_bytes).unwrap_err() {
                RequestError::ConnectionIdTooShort => {}
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod sending_data_over_a_new_connection {
        use super::*;

        #[test]
        fn returns_error_when_address_length_is_too_short() {
            let request_bytes1 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8].to_vec(); // 8 bytes connection id, 0 bytes address length (2 were expected)
            let request_bytes2 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0].to_vec(); // 8 bytes connection id, 1 bytes address length (2 were expected)

            match Request::try_from_bytes(&request_bytes1).unwrap_err() {
                RequestError::AddressLengthTooShort => {}
                _ => unreachable!(),
            }

            match Request::try_from_bytes(&request_bytes2).unwrap_err() {
                RequestError::AddressLengthTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_when_address_too_short_for_given_address_length() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 1].to_vec(); // 8 bytes connection id, 2 bytes address length, missing address
            match Request::try_from_bytes(&request_bytes).unwrap_err() {
                RequestError::AddressTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_for_when_return_address_is_too_short() {
            // this one has "foo.com" remote address and correct 8 bytes of connection_id
            let request_bytes_prefix = [
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
            ];

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            // take only part of actual recipient
            let request_bytes: Vec<_> = request_bytes_prefix
                .iter()
                .cloned()
                .chain(recipient_bytes.iter().take(40).cloned())
                .collect();

            match Request::try_from_bytes(&request_bytes).unwrap_err() {
                RequestError::ReturnAddressTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_for_when_return_address_is_malformed() {
            // this one has "foo.com" remote address and correct 8 bytes of connection_id
            let request_bytes_prefix = [
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
            ];

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let mut recipient_bytes = recipient.to_bytes();

            // mess up few bytes
            recipient_bytes[0] = 255;
            recipient_bytes[15] ^= 1;
            recipient_bytes[31] ^= 1;

            let request_bytes: Vec<_> = request_bytes_prefix
                .iter()
                .cloned()
                .chain(recipient_bytes.iter().cloned())
                .collect();
            assert!(Request::try_from_bytes(&request_bytes)
                .unwrap_err()
                .is_malformed_return());
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

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            let request_bytes: Vec<_> = request_bytes
                .into_iter()
                .chain(recipient_bytes.iter().cloned())
                .collect();

            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Connect(req) => {
                    assert_eq!("foo.com".to_string(), req.remote_addr);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), req.conn_id);
                    assert_eq!(
                        req.return_address.to_bytes().to_vec(),
                        recipient.to_bytes().to_vec()
                    );
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // this one has a 1-byte remote address, correct 8 bytes of connection_id, and 3 bytes request data
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

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            let request_bytes: Vec<_> = request_bytes
                .into_iter()
                .chain(recipient_bytes.iter().cloned())
                .collect();

            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Connect(req) => {
                    assert_eq!("foo.com".to_string(), req.remote_addr);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), req.conn_id);
                    assert_eq!(
                        req.return_address.to_bytes().to_vec(),
                        recipient.to_bytes().to_vec()
                    );
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
            // correct 8 bytes of connection_id, 1 byte of local_closed and 0 bytes request data
            let request_bytes = [RequestFlag::Send as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0].to_vec();
            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Send(conn_id, data, local_closed) => {
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(Vec::<u8>::new(), data);
                    assert!(!local_closed)
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // correct 8 bytes of connection_id, 1 byte of local_closed and 3 bytes request data (all 255)
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
                0,
                255,
                255,
                255,
            ]
            .to_vec();

            let request = Request::try_from_bytes(&request_bytes).unwrap();
            match request {
                Request::Send(conn_id, data, local_closed) => {
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), conn_id);
                    assert_eq!(vec![255, 255, 255], data);
                    assert!(!local_closed)
                }
                _ => unreachable!(),
            }
        }
    }
}
