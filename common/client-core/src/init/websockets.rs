use nym_http_api_client::dns::HickoryDnsResolver;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{
    error::{Error, UrlError},
    handshake::client::Response,
};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    use std::net::SocketAddr;

    let resolver = HickoryDnsResolver::default();

    let sock_addrs: Vec<SocketAddr> = resolver
        .resolve_str(endpoint)
        .await
        .map_err(|_| UrlError::NoPathOrQuery)? // failed to resolve
        .collect();

    let stream = TcpStream::connect(&sock_addrs[..]).await?;

    tokio_tungstenite::client_async_tls(endpoint, stream).await
}
