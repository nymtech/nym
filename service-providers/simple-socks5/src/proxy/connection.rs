use simple_socks5_requests::{ConnectionId, RemoteAddress};
use tokio::net::TcpStream;
use tokio::prelude::*;

#[derive(Debug)]
pub(crate) struct Connection {
    id: ConnectionId,
    address: RemoteAddress,
    conn: TcpStream,
}

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
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        initial_data: &[u8],
    ) -> io::Result<Self> {
        // TODO: do we want to have async stuff in constructor?
        let conn = TcpStream::connect(&address).await?;
        let mut connection = Connection { id, address, conn };
        connection.send_data(&initial_data).await?;
        Ok(connection)
    }

    pub(crate) async fn send_data(&mut self, data: &[u8]) -> io::Result<()> {
        self.conn.write_all(&data).await
    }

    // /// Runs the request, by setting up a new TCP connection, shooting request
    // /// data up that connection, and returning whatever it receives in response.
    // pub(crate) async fn run(&self) -> tokio::io::Result<Response> {
    //     // rename to connect
    //     println!(
    //         "connecting id {:?}, remote {:?}, data {:?}",
    //         self.id,
    //         self.address,
    //         String::from_utf8_lossy(&self.data)
    //     );

    //     let mut stream = TcpStream::connect(&self.address).await?;
    //     stream.write_all(&self.data).await?;

    //     let response_buf = Connection::try_read_response_data(&mut stream).await?;
    //     println!(
    //         "response data: {:?}",
    //         String::from_utf8_lossy(&response_buf)
    //     );
    //     let response = Response::new(self.id, response_buf);
    //     Ok(response)
    // }

    /// Read response data by looping, waiting for anything we get back from the
    /// remote server. Returns once it times out or the connection closes.
    pub(crate) async fn try_read_response_data(&mut self) -> io::Result<Vec<u8>> {
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
                read_data = self.conn.read(&mut buf) => {
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
