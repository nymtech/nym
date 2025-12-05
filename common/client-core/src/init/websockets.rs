use crate::error::ClientCoreError;

#[cfg(not(target_arch = "wasm32"))]
use nym_topology::EntryDetails;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tungstenite::handshake::client::Response;
use url::Url;

use std::net::SocketAddr;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn connect_async(
    endpoint: &EntryDetails,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), ClientCoreError> {
    let uri = ws_entry_address(endpoint, false)
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

pub fn ws_entry_address_tls(entry: &EntryDetails) -> Option<Url> {
    let hostname = entry.hostname.as_ref()?;
    let wss_port = entry.clients_wss_port?;

    Url::parse(&format!("wss://{hostname}:{wss_port}")).ok()
}

pub fn ws_entry_address_no_tls(entry: &EntryDetails, prefer_ipv6: bool) -> Option<Url> {
    if let Some(hostname) = entry.hostname.as_ref() {
        return Url::parse(&format!("ws://{hostname}:{}", entry.clients_ws_port)).ok();
    }

    if prefer_ipv6 {
        if let Some(ipv6) = entry.ip_addresses.iter().find(|ip| ip.is_ipv6()) {
            return Url::parse(&format!("ws://{ipv6}:{}", entry.clients_ws_port)).ok();
        }
    }

    let any_ip = entry.ip_addresses.first()?;
    Url::parse(&format!("ws://{any_ip}:{}", entry.clients_ws_port)).ok()
}

pub fn ws_entry_address(entry: &EntryDetails, prefer_ipv6: bool) -> Option<Url> {
    if let Some(tls) = ws_entry_address_tls(entry) {
        return Some(tls);
    }
    ws_entry_address_no_tls(entry, prefer_ipv6)
}
