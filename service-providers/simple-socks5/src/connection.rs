use futures::channel::mpsc;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use socks5_requests::{ConnectionId, RemoteAddress, Response};
use tokio::net::TcpStream;
use tokio::prelude::*;
use utils::connection_controller::ConnectionReceiver;
use utils::proxy_runner::ProxyRunner;

/// A TCP connection between the Socks5 service provider, which makes
/// outbound requests on behalf of users and returns the responses through
/// the mixnet.
#[derive(Debug)]
pub(crate) struct Connection {
    id: ConnectionId,
    address: RemoteAddress,
    conn: Option<TcpStream>,
    return_address: Recipient,
}

impl Connection {
    pub(crate) async fn new(
        id: ConnectionId,
        address: RemoteAddress,
        initial_data: &[u8],
        return_address: Recipient,
    ) -> io::Result<Self> {
        let mut conn = TcpStream::connect(&address).await?;

        // write the initial data to the connection before continuing
        info!(
            "Sending initial {} bytes to {}",
            initial_data.len(),
            address
        );
        conn.write_all(initial_data).await?;

        Ok(Connection {
            id,
            address,
            conn: Some(conn),
            return_address,
        })
    }

    pub(crate) async fn run_proxy(
        &mut self,
        mix_receiver: ConnectionReceiver,
        mix_sender: mpsc::UnboundedSender<(Response, Recipient)>,
    ) {
        let stream = self.conn.take().unwrap();
        let connection_id = self.id;
        let recipient = self.return_address;
        let (stream, _) = ProxyRunner::new(stream, mix_receiver, mix_sender, connection_id)
            .run(move |conn_id, read_data, socket_closed| {
                (Response::new(conn_id, read_data, socket_closed), recipient)
            })
            .await
            .into_inner();
        self.conn = Some(stream);
    }
}
