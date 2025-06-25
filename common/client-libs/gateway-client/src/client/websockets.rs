use crate::error::GatewayClientError;

#[cfg(unix)]
use std::{
    os::fd::{AsRawFd, RawFd},
    sync::Arc,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::Response;
use url::{Host, Url};

use std::net::SocketAddr;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &str,
    #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), GatewayClientError> {
    use tokio::net::TcpSocket;

    let uri =
        Url::parse(endpoint).map_err(|_| GatewayClientError::InvalidUrl(endpoint.to_owned()))?;
    let port: u16 = uri.port_or_known_default().unwrap_or(443);

    let host = uri
        .host()
        .ok_or(GatewayClientError::InvalidUrl(endpoint.to_owned()))?;

    // Get address for tcp connection, using system DNS resolver
    let sock_addrs: Vec<SocketAddr> = match host {
        Host::Ipv4(addr) => vec![SocketAddr::new(addr.into(), port)],
        Host::Ipv6(addr) => vec![SocketAddr::new(addr.into(), port)],
        Host::Domain(domain) => {
            // Do a DNS lookup for the domain using system DNS resolver
            tokio::net::lookup_host((domain, port))
                .await
                .map_err(|err| GatewayClientError::NetworkConnectionFailed {
                    address: endpoint.to_owned(),
                    source: err.into(),
                })?
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
            source: err.into(),
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
                    source: err.into(),
                });
                continue;
            }
        }
    }

    tokio_tungstenite::client_async_tls(endpoint, stream?)
        .await
        .map_err(|error| GatewayClientError::NetworkConnectionFailed {
            address: endpoint.to_owned(),
            source: error,
        })
}
