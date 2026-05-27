// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

#![doc = include_str!("ARCHITECTURE.md")]

mod bridge;
mod device;
mod error;
mod tunnel;

/// Error type for all fallible smolmix operations.
pub use error::SmolmixError;

/// The IPv4/IPv6 address pair allocated to this tunnel by the IPR.
pub use tunnel::IpPair;

/// A Nym mixnet address, used to target a specific IPR exit node.
pub use tunnel::Recipient;

/// A TCP stream routed through the mixnet. Implements `AsyncRead + AsyncWrite`.
///
/// Obtained via [`Tunnel::tcp_connect`]. Use it anywhere a
/// `tokio::net::TcpStream` is accepted: tokio-rustls, hyper, tokio-tungstenite,
/// and the rest of the async ecosystem.
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let tunnel = smolmix::Tunnel::new().await?;
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// let mut stream = tunnel.tcp_connect("1.1.1.1:80".parse()?).await?;
/// stream.write_all(b"GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nConnection: close\r\n\r\n").await?;
/// let mut buf = Vec::new();
/// stream.read_to_end(&mut buf).await?;
/// # Ok(())
/// # }
/// ```
pub use tunnel::TcpStream;

/// A mixnet tunnel providing TCP and UDP socket access.
pub use tunnel::Tunnel;

/// Builder for configuring and creating a [`Tunnel`].
///
/// See [`Tunnel::builder()`] for usage.
pub use tunnel::TunnelBuilder;

/// A UDP socket routed through the mixnet. Supports `send_to` / `recv_from`.
///
/// Obtained via [`Tunnel::udp_socket`] or [`Tunnel::udp_socket_on`].
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let tunnel = smolmix::Tunnel::new().await?;
/// let udp = tunnel.udp_socket().await?;
/// udp.send_to(b"hello", "1.1.1.1:9999".parse()?).await?;
///
/// let mut buf = [0u8; 1024];
/// let (len, _src) = udp.recv_from(&mut buf).await?;
/// # Ok(())
/// # }
/// ```
pub use tunnel::UdpSocket;
