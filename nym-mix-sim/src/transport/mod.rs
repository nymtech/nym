// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a participant sends and receives through, real or simulated.
//!
//! The simulator only ever makes two syscalls - one `sendto`, one non-blocking `recvfrom` - so a
//! participant's socket is exactly [`SimEndpoint`]. A live run binds a [`UdpSocket`]; a test run
//! hands out [`MemoryEndpoint`](memory::MemoryEndpoint)s from a
//! [`MemoryNetwork`](memory::MemoryNetwork) and never touches the network stack.

use std::{
    io::{self, ErrorKind},
    net::{SocketAddr, UdpSocket},
};

pub mod memory;

/// One endpoint's worth of the two syscalls the simulator makes.
///
/// Implementations keep the `io::Result` shape rather than an error type of their own, but report
/// "nothing waiting" as `Ok(None)` rather than as an error. A non-blocking socket signals that with
/// [`io::ErrorKind::WouldBlock`], which the [`UdpSocket`] impl folds into `Ok(None)` so that a
/// caller has one shape to match and an `Err` always means a real failure.
pub trait SimEndpoint: Send {
    /// Send `bytes` to `dst`, returning how many were accepted.
    fn send_to(&self, dst: SocketAddr, bytes: &[u8]) -> io::Result<usize>;

    /// Take one waiting datagram, truncated to `buf`.
    ///
    /// Never blocks: returns `Ok(None)` when nothing is waiting.
    fn try_recv_from(&self, buf: &mut [u8]) -> io::Result<Option<(usize, SocketAddr)>>;
}

impl SimEndpoint for UdpSocket {
    fn send_to(&self, dst: SocketAddr, bytes: &[u8]) -> io::Result<usize> {
        UdpSocket::send_to(self, bytes, dst)
    }

    fn try_recv_from(&self, buf: &mut [u8]) -> io::Result<Option<(usize, SocketAddr)>> {
        match UdpSocket::recv_from(self, buf) {
            Ok(r) => Ok(Some(r)),
            // Non-blocking call saying it has nothing
            Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Bind a non-blocking UDP socket, which is what every live participant runs on.
pub fn bind_udp(address: SocketAddr) -> anyhow::Result<Box<dyn SimEndpoint>> {
    let socket = UdpSocket::bind(address)?;
    socket.set_nonblocking(true)?;
    Ok(Box::new(socket))
}
