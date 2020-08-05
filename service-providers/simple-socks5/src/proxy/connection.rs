use crate::proxy::response::Response;
use tokio::net::TcpStream;
use tokio::prelude::*;

#[derive(Debug)]
pub(crate) struct Connection {
    id: Id,
    address: RemoteAddress,
    // TODO: replace data with stream?
    data: RequestData,
    conn: TcpStream,
}

pub(crate) type Id = [u8; 16];
type RemoteAddress = String;
type RequestData = Vec<u8>;

/*
    Request:
    Connect: CONN_FLAG || address_length || remote_address_bytes || connection_id || request_data_content (vec<u8>)
    Send: SEND_FLAG || connection_id || request_data_content (vec<u8>)
    Close: CLOSE_FLAG

    Mixnetwork -> request -> router -> connection -> .... tcp magic here ....
*/

// enum Connectionquest {
//     New(RequestId, RemoteAddress),Connection
//     SomethingSomethingExisting,Connection
//     ClosConnection
// }

impl Connection {
    /// Constructor: deserializes the incoming data and returns a new Connection
    /// which can be used to shoot data up and down.
    pub(crate) fn new(request_bytes: Vec<u8>) -> Connection {
        let (id, address, data) = Connection::deserialize(request_bytes);
        let conn = todo!();
        Connection {
            id,
            address,
            data,
            conn,
        }
    }

    async fn send_data(&mut self, data: &[u8]) -> io::Result<()> {
        self.conn.write_all(&data).await
    }

    /// Runs the request, by setting up a new TCP connection, shooting request
    /// data up that connection, and returning whatever it receives in response.
    pub(crate) async fn run(&self) -> tokio::io::Result<Response> {
        // rename to connect
        println!(
            "connecting id {:?}, remote {:?}, data {:?}",
            self.id,
            self.address,
            String::from_utf8_lossy(&self.data)
        );

        let mut stream = TcpStream::connect(&self.address).await?;
        stream.write_all(&self.data).await?;

        let response_buf = Connection::try_read_response_data(&mut stream).await?;
        println!(
            "response data: {:?}",
            String::from_utf8_lossy(&response_buf)
        );
        let response = Response::new(self.id, response_buf);
        Ok(response)
    }

    /// Read response data by looping, waiting for anything we get back from the
    /// remote server. Returns once it times out or the connection closes.
    async fn try_read_response_data<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
        let timeout_duration = std::time::Duration::from_millis(500);
        let mut data = Vec::new();
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
                        Err(err) => return Err(err),
                        Ok(0) => return Ok(data),
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
    /// ------------------------------------------------------------------------
    /// | address_length | remote_address_bytes | connection_id | request_data |
    /// |      2         |    address_length    |     16     |   ...           |
    /// ------------------------------------------------------------------------
    ///
    /// We return the useful deserialized values.
    fn deserialize(request_bytes: Vec<u8>) -> (Id, RemoteAddress, RequestData) {
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
        let connection_id = Connection::from_slice(&request_id_vec);

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
        Connection::deserialize(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_address_too_short_for_given_address_length() {
        let request_bytes = [0, 1].to_vec(); // there should be a 1-byte remote address, but there's nothing
        Connection::deserialize(request_bytes);
    }

    #[test]
    #[should_panic]
    fn panics_when_request_id_too_short() {
        // there is a 1-byte remote address, followed by only 1 byte of connection_id, which is too short (must be 16 bytes)
        let request_bytes = [0, 1, 0, 1].to_vec();
        Connection::deserialize(request_bytes);
    }
    #[test]
    fn works_when_request_is_sized_properly_even_without_data() {
        // this one has "foo.com" remote address, correct 16 bytes of connection_id, and 0 bytes request data
        let request_bytes = [
            0, 7, 102, 111, 111, 46, 99, 111, 109, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
            15, 16,
        ]
        .to_vec();
        let (id, remote_address, data) = Connection::deserialize(request_bytes);
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
        let (id, remote_address, data) = Connection::deserialize(request_bytes);
        assert_eq!("foo.com".to_string(), remote_address);
        assert_eq!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], id);
        assert_eq!(vec![255, 255, 255], data);
    }
}
