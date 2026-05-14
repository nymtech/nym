// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `futures::io` socket adapters over the smoltcp stack.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};
use smoltcp::iface::SocketHandle;
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::socket::udp as smoltcp_udp;
use smoltcp::wire::{IpAddress, IpEndpoint};

use crate::reactor::{ReactorNotify, SmoltcpStack};

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
