use nymsphinx::addressing::clients::Recipient;
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
    return_address: Recipient,
}

impl Connection {
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        initial_data: &[u8],
        return_address: Recipient,
    ) -> io::Result<Self> {
        let conn = match TcpStream::connect(&address).await {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("error while connecting to {:?} ! - {:?}", address, err);
                return Err(err);
            }
        };
        let mut connection = Connection {
            id,
            address,
            conn,
            return_address,
        };
        connection.send_data(&initial_data).await?;
        Ok(connection)
    }

    pub(crate) fn return_address(&self) -> Recipient {
        self.return_address.clone()
    }

    pub(crate) async fn send_data(&mut self, data: &[u8]) -> io::Result<()> {
        println!("Sending {} bytes to {}", data.len(), self.address);
        self.conn.write_all(&data).await
    }

    /// Read response data by looping, waiting for anything we get back from the
    /// remote server. Returns once it times out or the connection closes.
    pub(crate) async fn try_read_response_data(&mut self) -> io::Result<Vec<u8>> {
        let timeout_duration = std::time::Duration::from_millis(500);
        let mut data = Vec::new();
        let mut timeout = tokio::time::delay_for(timeout_duration);
        loop {
            let mut buf = [0u8; 8192];
            tokio::select! {
                _ = &mut timeout => {
                    return Ok(data) // we return all response data on timeout
                }
                read_data = self.conn.read(&mut buf) => {
                    match read_data {
                        Err(err) => return Err(err),
                        Ok(0) => {
                            return Ok(data)
                        }
                        Ok(n) => {
                            let now = timeout.deadline();
                            let next = now + timeout_duration;
                            timeout.reset(next);
                                    println!("Receiving {} bytes from {}", n, self.address);

                            data.extend_from_slice(&buf[..n])
                        }
                    }
                }
            }
        }
    }
}

// TODO: perhaps a smart implementation of this could alleviate some issues associated with `try_read_response_data` ?
// impl AsyncRead for Connection {
//     fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
//         unimplemented!()
//     }
// }
