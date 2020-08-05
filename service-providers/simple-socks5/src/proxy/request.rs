use crate::proxy::response::Response;
use tokio::net::TcpStream;
use tokio::prelude::*;

#[derive(Debug)]
pub(crate) struct Request {
    id: RequestId,
    address: RemoteAddress,
    data: RequestData,
}

pub(crate) type RequestId = [u8; 16];
type RemoteAddress = String;
type RequestData = Vec<u8>;

impl Request {
    /// Constructor: deserializes the incoming data and returns a new Request
    /// which can be run.
    pub(crate) fn new(request_bytes: Vec<u8>) -> Request {
        let (id, address, data) = Request::deserialize(request_bytes);
        Request { id, address, data }
    }

    /// Runs the request, by setting up a new TCP connection, shooting request
    /// data up that connection, and returning whatever it receives in response.
    pub(crate) async fn run(&self) -> tokio::io::Result<Response> {
        println!(
            "running request id {:?}, remote {:?}, data {:?}",
            self.id,
            self.address,
            String::from_utf8_lossy(&self.data)
        );

        let mut stream = TcpStream::connect(&self.address).await?;
        stream.write_all(&self.data).await?;

        let response_buf = Request::try_read_response_data(&mut stream).await?;
        println!(
            "response data: {:?}",
            String::from_utf8_lossy(&response_buf)
        );
        let response = Response::new(self.id, response_buf);
        Ok(response)
    }

    async fn try_read_response_data<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        let timeout_duration = std::time::Duration::from_millis(500);

        let mut timeout = tokio::time::delay_for(timeout_duration);
        loop {
            let mut buf = [0u8; 1024];
            tokio::select! {
                _ = &mut timeout => {
                    println!("we timed out!");
                    return Ok(data)
                }
                read_data = reader.read(&mut buf) => {
                    match read_data {
                        Ok(0) => return Ok(data),
                        Err(err) => return Err(err),
                        Ok(n) => {
                            let now = timeout.deadline();
                            let next = now + timeout_duration;
                            timeout.reset(next);
                            data.extend_from_slice(&buf[..n])
                        }
                    }
                }
            }
        }
    }

    /// Deserialize the destination address and port, the request id,
    /// and the request body from bytes. This is the reverse of SocksRequest::serialize.
    ///
    /// Serialized bytes looks like this:
    ///
    /// ----------------------------------------------------------------
    /// | address_length | remote_address_bytes | request_id | request |
    /// |      2         |    address_length    |     16     |   ...   |
    /// ----------------------------------------------------------------
    ///
    /// We return the useful deserialized values.
    fn deserialize(request_bytes: Vec<u8>) -> (RequestId, RemoteAddress, RequestData) {
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
        let request_id = Request::from_slice(&request_id_vec);

        let data_start = request_id_end;
        let mut data = Vec::new();
        if data_start <= total_length {
            data = request_bytes[data_start..].to_vec();
        }
        (request_id, address, data)
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
        Request::deserialize(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_address_too_short_for_given_address_length() {
        let request_bytes = [0, 1].to_vec(); // there should be a 1-byte remote address, but there's nothing
        Request::deserialize(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_request_id_too_short() {
        // there is a 1-byte remote address, followed by only 1 byte of request_id, which is too short (must be 16 bytes)
        let request_bytes = [0, 1, 0, 1].to_vec();
        Request::deserialize(request_bytes);
    }
    #[test]
    fn works_when_request_is_sized_properly_even_without_data() {
        // this one has "foo.com" remote address, correct 16 bytes of request_id, and 0 bytes request data
        let request_bytes = [
            0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16,
        ]
        .to_vec();
        let (id, remote_address, data) = Request::deserialize(request_bytes);
        assert_eq!("foo.com".to_string(), remote_address);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        assert_eq!(Vec::<u8>::new(), data);
    }

    #[test]
    fn works_when_request_is_sized_properly_and_has_data() {
        // this one has a 1-byte remote address, correct 16 bytes of request_id, and 3 bytes request data
        let request_bytes = [
            0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16, 255, 255, 255,
        ]
        .to_vec();
        let (id, remote_address, data) = Request::deserialize(request_bytes);
        assert_eq!("foo.com".to_string(), remote_address);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        assert_eq!(vec![255, 255, 255], data);
    }
}
