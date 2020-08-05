use crate::proxy::message_router::Controller;
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
pub(crate) type RemoteAddress = String;
pub(crate) type RequestData = Vec<u8>;

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
        let (id, address, data) = Controller::parse_message(request_bytes);
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
}
