// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `futures::io` socket adapters over the smoltcp stack.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicU16, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

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
/// The `Tls` variant compiles in only when `fetch` or `websocket` features are
/// enabled, since plaintext-only builds (the `dns`-only TS SDK package) don't
/// need a TLS stack at all.
///
/// `Tls` is intentionally inlined (~744 B) rather than boxed: the pool holds
/// at most one entry per (host, port), so total memory is bounded by distinct
/// origins visited over the tunnel's lifetime. Boxing would force every match
/// arm into a `Box`-deref dance for no real benefit at typical usage.
#[allow(clippy::large_enum_variant)]
pub(crate) enum PooledConn {
    #[cfg(any(feature = "fetch", feature = "websocket"))]
    Tls(crate::tls::MaybeCloseNotify<futures_rustls::client::TlsStream<WasmTcpStream>>),
    Plain(WasmTcpStream),
}

/// TCP stream over the WASM tunnel. Implements `futures::io::{AsyncRead, AsyncWrite}`.
pub struct WasmTcpStream {
    pub(crate) stack: SmoltcpStack,
    pub(crate) handle: SocketHandle,
    pub(crate) notify: ReactorNotify,
    /// Set once `socket.close()` has been called (via `poll_close` or
    /// `Drop`). Makes the close path idempotent.
    closed: bool,
}

/// UDP socket over the WASM tunnel. Used for DNS queries.
pub struct WasmUdpSocket {
    pub(crate) stack: SmoltcpStack,
    pub(crate) handle: SocketHandle,
    pub(crate) notify: ReactorNotify,
}

impl AsyncRead for WasmTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let handle = self.handle;
        let notify = &self.notify;
        self.stack.with(|s| {
            let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(handle);

            if socket.can_recv() {
                let n = socket
                    .recv_slice(buf)
                    .map_err(|e| io::Error::other(format!("{e}")))?;
                crate::util::debug_log!("[tcp:read] Ready({n})");
                // Notify reactor: recv_slice() frees rx buffer, needs a
                // prompt window update ACK to keep the sender flowing.
                notify.notify_one();
                Poll::Ready(Ok(n))
            } else if !socket.may_recv() {
                // Remote sent FIN (EOF). `may_recv()` is false for CloseWait,
                // LastAck, Closed, TimeWait (unlike `is_open()` which misses CloseWait).
                Poll::Ready(Ok(0))
            } else {
                crate::util::debug_log!(
                    "[tcp:read] Pending (state={:?}, buf={}, recv_queue={})",
                    socket.state(),
                    buf.len(),
                    socket.recv_queue(),
                );
                // smoltcp wakes this waker on any state change affecting `recv`,
                // including FIN/CloseWait transitions that produce EOF.
                socket.register_recv_waker(cx.waker());
                Poll::Pending
            }
        })
    }
}

impl AsyncWrite for WasmTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let handle = self.handle;
        let notify = &self.notify;
        self.stack.with(|s| {
            let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(handle);

            if socket.can_send() {
                let n = socket
                    .send_slice(buf)
                    .map_err(|e| io::Error::other(format!("{e}")))?;
                notify.notify_one();
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
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nudge the reactor so any queued tx data dispatches promptly rather
        // than waiting for the next `poll_delay` deadline.
        self.notify.notify_one();
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let handle = this.handle;
        let notify = &this.notify;
        let closed = &mut this.closed;
        this.stack.with(|s| {
            let socket = s.sockets.get_mut::<smoltcp_tcp::Socket>(handle);
            if !*closed {
                socket.close();
                *closed = true;
                notify.notify_one();
            }
            if socket.state() == smoltcp_tcp::State::Closed {
                Poll::Ready(Ok(()))
            } else {
                // Wake on state-change progress through the FIN/ACK exchange.
                socket.register_send_waker(cx.waker());
                Poll::Pending
            }
        })
    }
}

impl Unpin for WasmTcpStream {}

impl Drop for WasmTcpStream {
    fn drop(&mut self) {
        // Queue a FIN (vs `abort()` which sends RST). The
        // reactor's pending_removal sweep removes the handle once smoltcp
        // transitions through the FIN/ACK exchange to State::Closed.
        let handle = self.handle;
        self.stack.with(|s| {
            s.sockets.get_mut::<smoltcp_tcp::Socket>(handle).close();
            s.pending_removal.push(handle);
        });
        self.notify.notify_one();
    }
}

impl AsyncRead for PooledConn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            #[cfg(any(feature = "fetch", feature = "websocket"))]
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
            #[cfg(any(feature = "fetch", feature = "websocket"))]
            PooledConn::Tls(s) => Pin::new(s).poll_write(cx, buf),
            PooledConn::Plain(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(any(feature = "fetch", feature = "websocket"))]
            PooledConn::Tls(s) => Pin::new(s).poll_flush(cx),
            PooledConn::Plain(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            #[cfg(any(feature = "fetch", feature = "websocket"))]
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
            stack.with(|s| {
                let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

                if socket.can_send() {
                    socket
                        .send_slice(buf, endpoint)
                        .map_err(|e| io::Error::other(format!("{e}")))?;
                    notify.notify_one();
                    Poll::Ready(Ok(buf.len()))
                } else {
                    socket.register_send_waker(cx.waker());
                    Poll::Pending
                }
            })
        })
        .await
    }

    /// Receive a datagram, returning (bytes_read, source_address).
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let stack = self.stack.clone();
        let handle = self.handle;

        futures::future::poll_fn(move |cx| {
            stack.with(|s| {
                let socket = s.sockets.get_mut::<smoltcp_udp::Socket>(handle);

                if socket.can_recv() {
                    let (n, meta) = socket
                        .recv_slice(buf)
                        .map_err(|e| io::Error::other(format!("{e}")))?;
                    let src = from_smoltcp_endpoint(meta.endpoint);
                    Poll::Ready(Ok((n, src)))
                } else {
                    socket.register_recv_waker(cx.waker());
                    Poll::Pending
                }
            })
        })
        .await
    }
}

impl Drop for WasmUdpSocket {
    fn drop(&mut self) {
        let handle = self.handle;
        self.stack.with(|s| {
            s.sockets.remove(handle);
        });
    }
}

/// Process-wide ephemeral port counter, seeded at [`EPHEMERAL_PORT_START`].
static EPHEMERAL_PORT: AtomicU16 = AtomicU16::new(EPHEMERAL_PORT_START);

/// Allocate the next ephemeral port (wraps at `u16::MAX` back to
/// [`EPHEMERAL_PORT_START`]). Single-threaded wasm32 means a plain
/// load/store is race-free; the atomic exists for `Sync`.
pub(crate) fn allocate_port() -> u16 {
    let current = EPHEMERAL_PORT.load(Ordering::Relaxed);
    let next = if current == u16::MAX {
        EPHEMERAL_PORT_START
    } else {
        current + 1
    };
    EPHEMERAL_PORT.store(next, Ordering::Relaxed);
    current
}

/// Drop-bomb for a `SocketHandle` mid-flight in `tcp_connect`. If the caller
/// errors out before producing a `WasmTcpStream`, `Drop` removes the handle
/// from the `SocketSet`. On success, `defuse()` disarms the guard and hands
/// back the handle for ownership transfer into `WasmTcpStream`.
struct InflightSocket {
    stack: Option<SmoltcpStack>,
    handle: SocketHandle,
}

impl InflightSocket {
    fn defuse(mut self) -> SocketHandle {
        self.stack = None;
        self.handle
    }
}

impl Drop for InflightSocket {
    fn drop(&mut self) {
        if let Some(stack) = self.stack.take() {
            let handle = self.handle;
            stack.with(|s| {
                s.sockets.remove(handle);
            });
        }
    }
}

/// Open a TCP connection through the tunnel and wait for `Established`.
///
/// Used by `WasmTunnel::tcp_connect` and by the DNS resolver provider; both
/// draw from the process-wide [`EPHEMERAL_PORT`] counter so allocations
/// don't collide.
pub(crate) async fn tcp_connect(
    stack: SmoltcpStack,
    notify: ReactorNotify,
    addr: SocketAddr,
    keepalive: Duration,
    buffer_size: usize,
) -> io::Result<WasmTcpStream> {
    let remote = to_smoltcp_endpoint(addr);
    let local_port = allocate_port();
    // Caller-supplied buffer size, capped at u16::MAX (TCP window field width).
    let buf_size = buffer_size.min(u16::MAX as usize);
    let tcp_rx = smoltcp_tcp::SocketBuffer::new(vec![0; buf_size]);
    let tcp_tx = smoltcp_tcp::SocketBuffer::new(vec![0; buf_size]);
    let mut socket = smoltcp_tcp::Socket::new(tcp_rx, tcp_tx);
    socket.set_keep_alive(Some(smoltcp::time::Duration::from_millis(
        keepalive.as_millis() as u64,
    )));

    // Synchronous phase: add + connect under one lock. If `connect()` errors,
    // clean up immediately while we still hold the lock.
    let handle = stack.with(|s| -> io::Result<SocketHandle> {
        let handle = s.sockets.add(socket);
        let crate::reactor::SmoltcpStackInner { iface, sockets, .. } = s;
        if let Err(e) = sockets.get_mut::<smoltcp_tcp::Socket>(handle).connect(
            iface.context(),
            remote,
            local_port,
        ) {
            sockets.remove(handle);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{e:?}"),
            ));
        }
        Ok(handle)
    })?;

    notify.notify_one();

    // Async phase: any error past this point must remove the socket. The
    // guard removes it on drop; we defuse it once the stream is constructed.
    let guard = InflightSocket {
        stack: Some(stack.clone()),
        handle,
    };

    {
        let stack = stack.clone();
        futures::future::poll_fn(move |cx| {
            stack.with(|s| {
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
        })
        .await?;
    }

    Ok(WasmTcpStream {
        stack,
        handle: guard.defuse(),
        notify,
        closed: false,
    })
}

/// Create a UDP socket bound to a fresh ephemeral port.
pub(crate) fn create_udp_socket(
    stack: SmoltcpStack,
    notify: ReactorNotify,
) -> io::Result<WasmUdpSocket> {
    let local_port = allocate_port();
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

    let handle = stack.with(|s| s.sockets.add(socket));

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
