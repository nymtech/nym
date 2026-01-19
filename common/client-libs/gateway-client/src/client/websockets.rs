use crate::error::GatewayClientError;

use nym_topology::EntryDetails;
#[cfg(unix)]
use std::{
    os::fd::{AsRawFd, RawFd},
    sync::Arc,
};
use tokio::net::{TcpSocket, TcpStream};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::Response;

use std::net::SocketAddr;

#[cfg(target_arch = "wasm32")]
use wasm_utils::websocket::JSWebsocket;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &EntryDetails,
    #[cfg(unix)] connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), GatewayClientError> {
    let uri = endpoint
        .ws_entry_address(false)
        .ok_or(GatewayClientError::InvalidEndpoint(endpoint.to_string()))?;
    let port: u16 = uri.port_or_known_default().unwrap_or(443);

    let sock_addrs = endpoint
        .ip_addresses
        .iter()
        .map(|addr| SocketAddr::new(*addr, port));
    let uri_str = uri.to_string();

    let mut stream = Err(GatewayClientError::NoEndpointForConnection {
        address: uri_str.clone(),
    });
    for sock_addr in sock_addrs {
        let socket = if sock_addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .map_err(|err| GatewayClientError::NetworkConnectionFailed {
            address: uri_str.clone(),
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
                    address: uri_str.clone(),
                    source: Box::new(tungstenite::Error::from(err)),
                });
                continue;
            }
        }
    }

    tokio_tungstenite::client_async_tls(uri.clone(), stream?)
        .await
        .map_err(|error| GatewayClientError::NetworkConnectionFailed {
            address: uri_str.clone(),
            source: Box::new(error),
        })
}
