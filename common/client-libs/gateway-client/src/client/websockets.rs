#[cfg(not(target_arch = "wasm32"))]
use crate::client::GatewayListeners;
use crate::error::GatewayClientError;

use nym_http_api_client::HickoryDnsResolver;
#[cfg(unix)]
use std::{
    os::fd::{AsRawFd, RawFd},
    sync::Arc,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::Response;
use url::{Host, Url};

use std::{net::SocketAddr, time::Duration};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &str,
    #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), GatewayClientError> {
    use tokio::net::TcpSocket;

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
            println!("I'M TIRED BOSS");
            tokio::time::sleep(Duration::from_secs(20)).await;
            resolver
                .resolve_str(domain)
                .await?
                .map(|a| SocketAddr::new(a, port))
                .collect()
        }
    };

    let mut stream = Err(GatewayClientError::NoEndpointForConnection {
        address: endpoint.to_owned(),
    });
    for sock_addr in sock_addrs {
        let socket = if sock_addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .map_err(|err| GatewayClientError::NetworkConnectionFailed {
            address: endpoint.to_owned(),
            source: Box::new(tungstenite::Error::from(err)),
        })?;

        #[cfg(unix)]
        if let Some(callback) = connection_fd_callback.as_ref() {
            callback.as_ref()(socket.as_raw_fd());
        }

        match socket.connect(sock_addr).await {
            Ok(s) => {
                stream = Ok(s);
                break;
            }
            Err(err) => {
                stream = Err(GatewayClientError::NetworkConnectionFailed {
                    address: endpoint.to_owned(),
                    source: Box::new(tungstenite::Error::from(err)),
                });
                continue;
            }
        }
    }

    tokio_tungstenite::client_async_tls(endpoint, stream?)
        .await
        .map_err(|error| GatewayClientError::NetworkConnectionFailed {
            address: endpoint.to_owned(),
            source: Box::new(error),
        })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async_with_fallback(
    endpoints: &GatewayListeners,
    #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), GatewayClientError> {
    match connect_async(
        endpoints.primary.as_ref(),
        #[cfg(unix)]
        connection_fd_callback.clone(),
    )
    .await
    {
        Ok(inner) => Ok(inner),
        Err(e) => {
            if let Some(fallback) = &endpoints.fallback {
                tracing::warn!(
                    "Main endpoint failed {} : {e}, trying fallback : {fallback}",
                    endpoints.primary
                );
                connect_async(
                    fallback.as_ref(),
                    #[cfg(unix)]
                    connection_fd_callback,
                )
                .await
            } else {
                Err(e)
            }
        }
    }
}
