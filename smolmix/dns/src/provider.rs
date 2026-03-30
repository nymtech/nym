// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! hickory-resolver [`RuntimeProvider`] routing all DNS I/O through a [`Tunnel`].

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use hickory_proto::runtime::iocompat::AsyncIoTokioAsStd;
use hickory_proto::runtime::{RuntimeProvider, TokioHandle, TokioTime};
use hickory_proto::udp::DnsUdpSocket;

use smolmix::Tunnel;

/// UDP socket wrapper routing DNS queries through the tunnel.
///
/// Thin newtype around [`tokio_smoltcp::UdpSocket`] implementing hickory's
/// [`DnsUdpSocket`] trait. The poll methods delegate directly since the
/// signatures are identical.
pub struct SmolmixUdpSocket(tokio_smoltcp::UdpSocket);

impl DnsUdpSocket for SmolmixUdpSocket {
    type Time = TokioTime;

    fn poll_recv_from(
        &self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<(usize, SocketAddr)>> {
        self.0.poll_recv_from(cx, buf)
    }

    fn poll_send_to(
        &self,
        cx: &mut Context<'_>,
        buf: &[u8],
        target: SocketAddr,
    ) -> Poll<io::Result<usize>> {
        self.0.poll_send_to(cx, buf, target)
    }
}

/// Runtime provider that routes all DNS I/O through a [`Tunnel`].
///
/// Implements hickory's [`RuntimeProvider`] so the resolver sends TCP and UDP DNS
/// traffic over the mixnet instead of the local network.
#[derive(Clone)]
pub struct SmolmixRuntimeProvider {
    pub(crate) tunnel: Tunnel,
    pub(crate) handle: TokioHandle,
}

impl RuntimeProvider for SmolmixRuntimeProvider {
    type Handle = TokioHandle;
    type Timer = TokioTime;
    type Udp = SmolmixUdpSocket;
    type Tcp = AsyncIoTokioAsStd<tokio_smoltcp::TcpStream>;

    fn create_handle(&self) -> Self::Handle {
        self.handle.clone()
    }

    fn connect_tcp(
        &self,
        server_addr: SocketAddr,
        _bind_addr: Option<SocketAddr>,
        _timeout: Option<std::time::Duration>,
    ) -> Pin<Box<dyn Send + Future<Output = io::Result<Self::Tcp>>>> {
        let tunnel = self.tunnel.clone();
        Box::pin(async move {
            let tcp = tunnel
                .tcp_connect(server_addr)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(AsyncIoTokioAsStd(tcp))
        })
    }

    fn bind_udp(
        &self,
        _local_addr: SocketAddr,
        _server_addr: SocketAddr,
    ) -> Pin<Box<dyn Send + Future<Output = io::Result<Self::Udp>>>> {
        let tunnel = self.tunnel.clone();
        Box::pin(async move {
            let udp = tunnel
                .udp_socket()
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(SmolmixUdpSocket(udp))
        })
    }
}
