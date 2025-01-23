use crate::error::GatewayClientError;

use nym_http_api_client::HickoryDnsResolver;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::{error::UrlError, handshake::client::Response};
use url::{Host, Url};

use std::net::SocketAddr;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), GatewayClientError> {
    let resolver = HickoryDnsResolver::default();
    let uri =
        Url::parse(endpoint).map_err(|_| GatewayClientError::InvalidUrl(endpoint.to_owned()))?;
    let port: u16 = uri.port_or_known_default().unwrap_or(443);

    let host = uri
        .host()
        .ok_or(GatewayClientError::InvalidUrl(endpoint.to_owned()))?;

    // Get address for tcp connection, if a domain is provided use our preferred resolver rather than
    // the default std resolve
    let sock_addrs: Vec<SocketAddr> = match host {
        Host::Ipv4(addr) => vec![SocketAddr::new(addr.into(), port)],
        Host::Ipv6(addr) => vec![SocketAddr::new(addr.into(), port)],
        Host::Domain(domain) => {
            // Do a DNS lookup for the domain using our custom DNS resolver
            resolver
                .resolve_str(domain)
                .await
                .map_err(|_| {
                    // failed to resolve
                    GatewayClientError::NetworkConnectionFailed {
                        address: endpoint.to_owned(),
                        source: UrlError::NoPathOrQuery.into(),
                    }
                })?
                .into_iter()
                .map(|a| SocketAddr::new(a, port))
                .collect()
        }
    };

    let stream = TcpStream::connect(&sock_addrs[..]).await.map_err(|error| {
        GatewayClientError::NetworkConnectionFailed {
            address: endpoint.to_owned(),
            source: error.into(),
        }
    })?;

    tokio_tungstenite::client_async_tls(endpoint, stream)
        .await
        .map_err(|error| GatewayClientError::NetworkConnectionFailed {
            address: endpoint.to_owned(),
            source: error,
        })
}
