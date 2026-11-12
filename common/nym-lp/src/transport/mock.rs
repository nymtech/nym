// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// unwrap in mock code is fine
#![allow(clippy::unwrap_used)]

use crate::transport::error::LpTransportError;
use crate::transport::traits::{
    LpHandshakeChannel, LpTransportChannel, read_n_bytes_async_read,
    receive_length_prefixed_bytes_async_read, send_serialised_packet_async_write,
    write_all_and_flush_async_write,
};
use async_trait::async_trait;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_test_utils::mocks::async_read_write::MockIOStream;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::fd::RawFd;

// per thread, so tests sharing addresses do not see each other's; a test and the handlers it
// spawns share a thread only on a current-thread runtime
thread_local! {
    /// What `connect` returns on this thread, by endpoint.
    static EXPECTED_CONNECTIONS: RefCell<HashMap<SocketAddr, MockTransport>> = RefCell::default();
}

/// An in-memory transport: one end of a `MockIOStream` pair.
#[derive(Default)]
pub struct MockTransport {
    stream: MockIOStream,
}

impl MockTransport {
    /// The other end of this transport's stream.
    pub fn try_get_remote_handle(&self) -> Self {
        MockTransport {
            stream: self.stream.try_get_remote_handle(),
        }
    }

    pub fn set_id(&self, id: u8) {
        self.stream.set_id(id)
    }

    /// Register what `connect(addr)` returns on this thread; gives back its remote end.
    pub fn expect_connection(addr: SocketAddr) -> Self {
        let transport = MockTransport::default();
        let remote = transport.try_get_remote_handle();
        EXPECTED_CONNECTIONS.with_borrow_mut(|expected| expected.insert(addr, transport));
        remote
    }

    /// Take the transport registered for `addr` by [`Self::expect_connection`], if any.
    fn take_expected_connection(addr: SocketAddr) -> Option<Self> {
        EXPECTED_CONNECTIONS.with_borrow_mut(|expected| expected.remove(&addr))
    }
}

#[async_trait]
impl LpHandshakeChannel for MockTransport {
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError> {
        self.stream.write_all_and_flush(data).await
    }

    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError> {
        self.stream.read_n_bytes(n).await
    }
}

#[async_trait]
impl LpTransportChannel for MockTransport {
    /// The transport registered for `endpoint`, or a loose end nothing holds the other side of.
    async fn connect_from(
        endpoint: SocketAddr,
        _local: Option<SocketAddr>,
    ) -> Result<Self, LpTransportError> {
        Ok(Self::take_expected_connection(endpoint).unwrap_or_default())
    }

    #[cfg(unix)]
    async fn connect_with_fd_callback(
        endpoint: SocketAddr,
        _connection_fd_callback: &(dyn Fn(RawFd) + Send + Sync),
    ) -> Result<Self, LpTransportError> {
        Self::connect_from(endpoint, None).await
    }

    fn set_no_delay(&mut self, _nodelay: bool) -> Result<(), LpTransportError> {
        Ok(())
    }

    async fn send_length_prefixed_transport_packet(
        &mut self,
        packet: &EncryptedLpPacket,
    ) -> Result<(), LpTransportError> {
        send_serialised_packet_async_write(&mut self.stream, packet).await
    }

    async fn receive_length_prefixed_transport_bytes(
        &mut self,
    ) -> Result<Vec<u8>, LpTransportError> {
        receive_length_prefixed_bytes_async_read(&mut self.stream).await
    }
}

#[async_trait]
impl LpHandshakeChannel for MockIOStream {
    async fn write_all_and_flush(&mut self, data: &[u8]) -> Result<(), LpTransportError> {
        write_all_and_flush_async_write(self, data).await
    }

    async fn read_n_bytes(&mut self, n: usize) -> Result<Vec<u8>, LpTransportError> {
        read_n_bytes_async_read(self, n).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_expected_connection_is_what_connect_returns() {
        let addr: SocketAddr = "127.0.0.1:4242".parse().unwrap();

        let mut remote = MockTransport::expect_connection(addr);
        let mut local = MockTransport::connect(addr).await.unwrap();

        local.write_all_and_flush(b"hi").await.unwrap();
        assert_eq!(remote.read_n_bytes(2).await.unwrap(), b"hi");

        // one registration, one connection
        assert!(MockTransport::take_expected_connection(addr).is_none());
    }
}
