use simple_socks5_requests::{ConnectionId, RemoteAddress};
use tokio::net::TcpStream;
use tokio::prelude::*;

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
        // TODO: do we want to have async stuff in constructor?
        let conn = TcpStream::connect(&address).await?;
        let mut connection = Connection { id, address, conn };
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
