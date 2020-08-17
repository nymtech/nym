use simple_socks5_requests::{ConnectionId, RemoteAddress};
use tokio::net::TcpStream;
use tokio::prelude::*;

/// A TCP connection between the Socks5 service provider, which makes
/// outbound requests on behalf of users and returns the responses through
/// the mixnet.
#[derive(Debug)]
pub(crate) struct Connection {
    id: ConnectionId,
    address: RemoteAddress,
    conn: TcpStream,
}

impl Connection {
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        initial_data: &[u8],
    ) -> io::Result<Self> {
        println!("Connecting to {}", address);
        let conn = match TcpStream::connect(&address).await {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("error while connecting! - {:?}", err);
                return Err(err);
            }
        };
        let mut connection = Connection { id, address, conn };
        println!("Sending data {:?}", initial_data);
        connection.send_data(&initial_data).await?;
        Ok(connection)
    }

    pub(crate) async fn send_data(&mut self, data: &[u8]) -> io::Result<()> {
        self.conn.write_all(&data).await
    }

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
                    eprintln!("we timed out - read {:?}", data);
                    return Ok(data) // we return all response data on timeout
                }
                read_data = self.conn.read(&mut buf) => {
                    match read_data {
                        Err(err) => return Err(err),
                        Ok(0) => {
                            eprintln!("read 0 - connection is closed (accumulated {:?})", data);
                            return Ok(data)
                        }
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
