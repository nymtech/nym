// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `futures::io` socket adapters over the smoltcp stack.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::socket::udp as smoltcp_udp;
use smoltcp::wire::{IpAddress, IpEndpoint};

use crate::reactor::{ReactorNotify, SmoltcpStack};

/// First port in the ephemeral range. Per IANA, 49152-65535 is the dynamic /
/// private range with no IANA-assigned services, safe for client sockets.
pub(crate) const EPHEMERAL_PORT_START: u16 = 49152;

/// A pooled connection (TLS or plain TCP). Delegates `AsyncRead + AsyncWrite`.
pub(crate) enum PooledConn {
    Tls(futures_rustls::client::TlsStream<WasmTcpStream>),
    Plain(WasmTcpStream),
}

/// TCP stream over the WASM tunnel. Implements `futures::io::{AsyncRead, AsyncWrite}`.
pub struct WasmTcpStream {
    pub(crate) stack: Arc<Mutex<SmoltcpStack>>,
    pub(crate) handle: SocketHandle,
    pub(crate) notify: ReactorNotify,
}

/// UDP socket over the WASM tunnel. Used for DNS queries.
pub struct WasmUdpSocket {
    pub(crate) stack: Arc<Mutex<SmoltcpStack>>,
    pub(crate) handle: SocketHandle,
    pub(crate) notify: ReactorNotify,
}

impl AsyncRead for WasmTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);

        if socket.can_recv() {
            let n = socket
                .recv_slice(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            crate::util::debug_log!("[tcp:read] Ready({n})");
            // Notify reactor: recv_slice() frees rx buffer, needs a
            // prompt window update ACK to keep the sender flowing.
            let _ = self.notify.unbounded_send(());
            Poll::Ready(Ok(n))
        } else if !socket.may_recv() {
            // Remote sent FIN (EOF). `may_recv()` is false for CloseWait,
            // LastAck, Closed, TimeWait (unlike `is_open()` which misses CloseWait).
            Poll::Ready(Ok(0))
        } else {
            crate::util::debug_log!(
                "[tcp:read] Pending (state={:?}, buf={})",
                socket.state(),
                buf.len(),
            );
            // smoltcp wakes this waker on any state change affecting `recv`,
            // including FIN/CloseWait transitions that produce EOF.
            socket.register_recv_waker(cx.waker());
            Poll::Pending
        }
    }
}

impl AsyncWrite for WasmTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);

        if socket.can_send() {
            let n = socket
                .send_slice(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            let _ = self.notify.unbounded_send(());
            Poll::Ready(Ok(n))
        } else if !socket.is_open() {
            Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "socket closed",
            )))
        } else {
            socket.register_send_waker(cx.waker());
            Poll::Pending
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nudge the reactor so any queued tx data dispatches promptly rather
        // than waiting for the next `poll_delay` deadline.
        let _ = self.notify.unbounded_send(());
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut s = self.stack.lock().unwrap();
        let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(self.handle);
        socket.close();
        let _ = self.notify.unbounded_send(());
        Poll::Ready(Ok(()))
    }
}

impl Unpin for WasmTcpStream {}

impl Drop for WasmTcpStream {
    fn drop(&mut self) {
        let mut s = self.stack.lock().unwrap();
        s.sockets
            .get_mut::<smoltcp_tcp::Socket>(self.handle)
            .abort();
        s.sockets.remove(self.handle);
    }
}

impl AsyncRead for PooledConn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_read(cx, buf),
            PooledConn::Plain(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for PooledConn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_write(cx, buf),
            PooledConn::Plain(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_flush(cx),
            PooledConn::Plain(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PooledConn::Tls(s) => Pin::new(s).poll_close(cx),
            PooledConn::Plain(s) => Pin::new(s).poll_close(cx),
        }
    }
}

impl Unpin for PooledConn {}

impl WasmUdpSocket {
    /// Send a datagram to the given address.
    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        let endpoint = to_smoltcp_endpoint(target);
        let stack = self.stack.clone();
        let handle = self.handle;
        let notify = self.notify.clone();

        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

            if socket.can_send() {
                socket
                    .send_slice(buf, endpoint)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                let _ = notify.unbounded_send(());
                Poll::Ready(Ok(buf.len()))
            } else {
                socket.register_send_waker(cx.waker());
                Poll::Pending
            }
        })
        .await
    }

    /// Receive a datagram, returning (bytes_read, source_address).
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let stack = self.stack.clone();
        let handle = self.handle;

        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

            if socket.can_recv() {
                let (n, meta) = socket
                    .recv_slice(buf)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                let src = from_smoltcp_endpoint(meta.endpoint);
                Poll::Ready(Ok((n, src)))
            } else {
                socket.register_recv_waker(cx.waker());
                Poll::Pending
            }
        })
        .await
    }
}

impl Drop for WasmUdpSocket {
    fn drop(&mut self) {
        let mut s = self.stack.lock().unwrap();
        s.sockets.remove(self.handle);
    }
}

// === Socket-creation helpers ===
//
// These are free functions rather than methods on `WasmTunnel` so the DNS
// resolver provider in `dns.rs` can construct sockets without holding back a
// reference to the whole tunnel. The tunnel's `tcp_connect` / `udp_socket`
// methods now delegate to these.

/// Construct a fresh `Arc<AtomicU16>` ephemeral port counter, seeded at
/// [`EPHEMERAL_PORT_START`].
pub(crate) fn new_port_counter() -> Arc<AtomicU16> {
    Arc::new(AtomicU16::new(EPHEMERAL_PORT_START))
}

/// Allocate the next ephemeral port (wraps at `u16::MAX` back to
/// [`EPHEMERAL_PORT_START`]). Single-threaded wasm32 means a plain
/// load/store is race-free; the atomic exists for `Sync`.
pub(crate) fn allocate_port(next_port: &Arc<AtomicU16>) -> u16 {
    let current = next_port.load(Ordering::Relaxed);
    let next = if current >= u16::MAX {
        EPHEMERAL_PORT_START
    } else {
        current + 1
    };
    next_port.store(next, Ordering::Relaxed);
    current
}

/// Open a TCP connection through the tunnel and wait for `Established`.
///
/// Used by `WasmTunnel::tcp_connect` and by the DNS resolver provider; both
/// share one `next_port` counter via `Arc<AtomicU16>` so allocations don't
/// collide.
pub(crate) async fn tcp_connect(
    stack: Arc<Mutex<SmoltcpStack>>,
    notify: ReactorNotify,
    next_port: &Arc<AtomicU16>,
    addr: SocketAddr,
) -> io::Result<WasmTcpStream> {
    let remote = to_smoltcp_endpoint(addr);
    let local_port = allocate_port(next_port);
    let tcp_rx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
    let tcp_tx = smoltcp_tcp::SocketBuffer::new(vec![0; 65536]);
    let mut socket = smoltcp_tcp::Socket::new(tcp_rx, tcp_tx);
    socket.set_keep_alive(Some(smoltcp::time::Duration::from_millis(10_000)));

    let handle = {
        let mut s = stack.lock().unwrap();
        let handle = s.sockets.add(socket);
        let SmoltcpStack {
            ref mut iface,
            ref mut sockets,
            ..
        } = *s;
        sockets
            .get_mut::<smoltcp_tcp::Socket>(handle)
            .connect(iface.context(), remote, local_port)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e:?}")))?;
        handle
    };

    let _ = notify.unbounded_send(());

    {
        let stack = stack.clone();
        futures::future::poll_fn(move |cx| {
            let mut s = stack.lock().unwrap();
            let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(handle);
            match socket.state() {
                smoltcp_tcp::State::Established | smoltcp_tcp::State::CloseWait => {
                    Poll::Ready(Ok(()))
                }
                smoltcp_tcp::State::Closed => {
                    crate::util::debug_error!("[stream] TCP state: Closed, connection failed");
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        "TCP connection failed",
                    )))
                }
                _ => {
                    socket.register_recv_waker(cx.waker());
                    Poll::Pending
                }
            }
        })
        .await?;
    }

    Ok(WasmTcpStream {
        stack,
        handle,
        notify,
    })
}

/// Create a UDP socket bound to a fresh ephemeral port.
pub(crate) fn create_udp_socket(
    stack: Arc<Mutex<SmoltcpStack>>,
    notify: ReactorNotify,
    next_port: &Arc<AtomicU16>,
) -> io::Result<WasmUdpSocket> {
    let local_port = allocate_port(next_port);
    let udp_rx = smoltcp_udp::PacketBuffer::new(
        vec![smoltcp_udp::PacketMetadata::EMPTY; 16],
        vec![0; 65535],
    );
    let udp_tx = smoltcp_udp::PacketBuffer::new(
        vec![smoltcp_udp::PacketMetadata::EMPTY; 16],
        vec![0; 65535],
    );
    let mut socket = smoltcp_udp::Socket::new(udp_rx, udp_tx);
    socket
        .bind(local_port)
        .map_err(|_| io::Error::new(io::ErrorKind::AddrInUse, "UDP bind failed"))?;

    let handle = {
        let mut s = stack.lock().unwrap();
        s.sockets.add(socket)
    };

    Ok(WasmUdpSocket {
        stack,
        handle,
        notify,
    })
}

// Address conversion helpers

pub(crate) fn to_smoltcp_endpoint(addr: SocketAddr) -> IpEndpoint {
    let ip = match addr.ip() {
        IpAddr::V4(v4) => IpAddress::Ipv4(v4),
        IpAddr::V6(v6) => IpAddress::Ipv6(v6),
    };
    IpEndpoint::new(ip, addr.port())
}

pub(crate) fn from_smoltcp_endpoint(ep: IpEndpoint) -> SocketAddr {
    let ip = match ep.addr {
        IpAddress::Ipv4(v4) => IpAddr::V4(v4),
        IpAddress::Ipv6(v6) => IpAddr::V6(v6),
    };
    SocketAddr::new(ip, ep.port)
}
