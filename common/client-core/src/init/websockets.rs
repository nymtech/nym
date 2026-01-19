use crate::error::ClientCoreError;

#[cfg(not(target_arch = "wasm32"))]
use nym_topology::EntryDetails;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::Response;

use std::net::SocketAddr;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &EntryDetails,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), ClientCoreError> {
    let uri = endpoint
        .ws_entry_address(false)
        .ok_or(ClientCoreError::InvalidEndpoint(endpoint.to_string()))?;
    let port: u16 = uri.port_or_known_default().unwrap_or(443);

    let sock_addrs: Vec<SocketAddr> = endpoint
        .ip_addresses
        .iter()
        .map(|addr| SocketAddr::new(addr.clone(), port))
        .collect();

    let stream = TcpStream::connect(&sock_addrs[..]).await?;

    tokio_tungstenite::client_async_tls(uri.to_string(), stream)
        .await
        .map_err(Into::into)
}
