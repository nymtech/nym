use nymsphinx::addressing::clients::Recipient;
use simple_socks5_requests::{ConnectionId, RemoteAddress};
use tokio::net::TcpStream;
use tokio::prelude::*;
use utils::read_delay_loop::try_read_data;

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
        try_read_data(timeout_duration, &mut self.conn, &self.address).await
    }
}
