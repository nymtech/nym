// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! WebSocket connections through the Nym mixnet.
//!
//! This crate wraps [tokio-tungstenite] with a convenience [`connect()`]
//! function that handles DNS resolution, TCP, TLS, and the WebSocket upgrade
//! — all through a smolmix [`Tunnel`].
//!
//! # Quick start
//!
//! ```ignore
//! use smolmix_tungstenite::connect;
//! use futures::{SinkExt, StreamExt};
//!
//! let tunnel = smolmix::Tunnel::new().await?;
//! let (mut ws, _resp) = connect(&tunnel, "wss://echo.websocket.org").await?;
//!
//! ws.send(Message::Text("hello".into())).await?;
//! let reply = ws.next().await.ok_or("no reply")??;
//! ```

use std::io;

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::Error as WsError;
use tokio_tungstenite::tungstenite::http::Response as WsResponse;
use tokio_tungstenite::WebSocketStream;

use smolmix::Tunnel;

// Re-exports so users don't need tokio-tungstenite or tungstenite in their Cargo.toml
pub use tokio_tungstenite::tungstenite;
pub use tokio_tungstenite::tungstenite::Message;

/// The WebSocket stream type returned by [`connect()`].
///
/// This is a `WebSocketStream` over a TLS-wrapped TCP connection through the
/// tunnel. Use it with `futures::SinkExt` and `futures::StreamExt`.
pub type WsStream = WebSocketStream<smolmix_tls::TlsStream<tokio_smoltcp::TcpStream>>;

/// Connect to a WebSocket server through the tunnel.
///
/// Handles DNS resolution, TCP connection, TLS handshake, and WebSocket
/// upgrade — all through the mixnet. Only `wss://` URLs are supported.
///
/// Returns the WebSocket stream and the HTTP upgrade response.
pub async fn connect<R>(
    tunnel: &Tunnel,
    request: R,
) -> Result<(WsStream, WsResponse<Option<Vec<u8>>>), Error>
where
    R: IntoClientRequest + Unpin,
{
    let request = request.into_client_request().map_err(Error::WebSocket)?;

    let uri = request.uri();
    let scheme = uri.scheme_str().unwrap_or("wss");
    if scheme != "wss" {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only wss:// URLs are supported (use raw TCP for unencrypted WebSocket)",
        )));
    }

    let host = uri
        .host()
        .ok_or_else(|| {
            Error::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "URI missing host",
            ))
        })?
        .to_owned();
    let port = uri.port_u16().unwrap_or(443);

    // DNS through the tunnel
    let addrs = smolmix_dns::resolve(tunnel, &host, port)
        .await
        .map_err(Error::Io)?;
    let addr = addrs.into_iter().next().ok_or_else(|| {
        Error::Io(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no addresses",
        ))
    })?;

    // TCP through the tunnel
    let tcp = tunnel
        .tcp_connect(addr)
        .await
        .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e)))?;

    // TLS handshake
    let tls_stream = smolmix_tls::connect(tcp, &host).await.map_err(Error::Io)?;

    // WebSocket upgrade
    let (ws, resp) = tokio_tungstenite::client_async(request, tls_stream)
        .await
        .map_err(Error::WebSocket)?;

    Ok((ws, resp))
}

/// Error type for WebSocket connections through the tunnel.
#[derive(Debug)]
pub enum Error {
    /// I/O error (DNS, TCP, or TLS).
    Io(io::Error),
    /// WebSocket protocol error.
    WebSocket(WsError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::WebSocket(e) => write!(f, "WebSocket error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::WebSocket(e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<WsError> for Error {
    fn from(e: WsError) -> Self {
        Error::WebSocket(e)
    }
}
