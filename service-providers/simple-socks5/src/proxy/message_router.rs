use crate::proxy::connection;

pub(crate) struct Controller {}

impl Controller {
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
    pub(crate) fn parse_message(
        request_bytes: Vec<u8>,
    ) -> (
        connection::Id,
        connection::RemoteAddress,
        connection::RequestData,
    ) {
        let total_length = request_bytes.len();
        let address_length: usize =
            (((request_bytes[0] as u16) << 8) | request_bytes[1] as u16).into(); // combines first 2 bytes into one u16
        let address_start = 2;
        let address_end = address_start + address_length;
        let address_vec = request_bytes[address_start..address_end].to_vec();
        let address = String::from_utf8_lossy(&address_vec).to_string();

        let request_id_start = address_end;
        let request_id_end = request_id_start + 16;
        let request_id_vec = request_bytes[request_id_start..request_id_end].to_vec();
        let connection_id = Controller::from_slice(&request_id_vec);

        let data_start = request_id_end;
        let mut data = Vec::new();
        if data_start <= total_length {
            data = request_bytes[data_start..].to_vec();
        }
        (connection_id, address, data)
    }

    fn from_slice(bytes: &[u8]) -> [u8; 16] {
        let mut array = [0; 16];
        let bytes = &bytes[..array.len()]; // panics if not enough data
        array.copy_from_slice(bytes);
        array
    }
}

#[cfg(test)]
mod deserialization_tests {
    use super::*;

    #[test]
    #[should_panic]
    fn panics_with_zero_bytes() {
        let request_bytes = Vec::new();
        Controller::parse_message(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_address_too_short_for_given_address_length() {
        let request_bytes = [0, 1].to_vec(); // there should be a 1-byte remote address, but there's nothing
        Controller::parse_message(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_request_id_too_short() {
        // there is a 1-byte remote address, followed by only 1 byte of connection_id, which is too short (must be 16 bytes)
        let request_bytes = [0, 1, 0, 1].to_vec();
        Controller::parse_message(request_bytes);
    }
    #[test]
    fn works_when_request_is_sized_properly_even_without_data() {
        // this one has "foo.com" remote address, correct 16 bytes of connection_id, and 0 bytes request data
        let request_bytes = [
            0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16,
        ]
        .to_vec();
        let (id, remote_address, data) = Controller::parse_message(request_bytes);
        assert_eq!("foo.com".to_string(), remote_address);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        assert_eq!(Vec::<u8>::new(), data);
    }

    #[test]
    fn works_when_request_is_sized_properly_and_has_data() {
        // this one has a 1-byte remote address, correct 16 bytes of connection_id, and 3 bytes request data
        let request_bytes = [
            0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 255, 255, 255,
        ]
        .to_vec();
        let (id, remote_address, data) = Controller::parse_message(request_bytes);
        assert_eq!("foo.com".to_string(), remote_address);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        assert_eq!(vec![255, 255, 255], data);
    }
}
