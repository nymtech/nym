// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// unwrap in mock code is fine
#![allow(clippy::unwrap_used)]

use crate::transport::error::LpTransportError;
use crate::transport::traits::{
    LpHandshakeChannel, LpTransportChannel, receive_length_prefixed_bytes_async_read,
    send_serialised_packet_async_write,
};
use async_trait::async_trait;
use nym_lp_data::packet::EncryptedLpPacket;
use nym_test_utils::mocks::async_read_write::MockIOStream;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// An in-memory transport: one end of a `MockIOStream` pair, plus the connections it will open.
#[derive(Default)]
pub struct MockTransport {
    stream: MockIOStream,

    /// What `connect_to` from either end of this pair returns, by endpoint.
    expected_connections: Arc<Mutex<HashMap<SocketAddr, MockTransport>>>,
}

impl MockTransport {
    /// The other end of this transport's stream, sharing its expected connections.
    pub fn try_get_remote_handle(&self) -> Self {
        MockTransport {
            stream: self.stream.try_get_remote_handle(),
            expected_connections: self.expected_connections.clone(),
        }
    }

    pub fn set_id(&self, id: u8) {
        self.stream.set_id(id)
    }

    /// Register what `connect_to(addr)` from either end of this pair returns; gives back its remote end.
    pub fn expect_connection(&self, addr: SocketAddr) -> Self {
        let transport = MockTransport::default();
        let remote = transport.try_get_remote_handle();
        self.expected_connections
            .lock()
            .unwrap()
            .insert(addr, transport);
        remote
    }

    /// Take the transport registered for `addr` by [`Self::expect_connection`], if any.
    pub fn take_expected_connection(&self, addr: SocketAddr) -> Option<Self> {
        self.expected_connections.lock().unwrap().remove(&addr)
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
    /// A loose end: nothing holds the other side.
    async fn connect_from(
        _endpoint: SocketAddr,
        _local: Option<SocketAddr>,
    ) -> Result<Self, LpTransportError> {
        Ok(MockTransport::default())
    }

    async fn connect_to(&mut self, endpoint: SocketAddr) -> Result<Self, LpTransportError> {
        Ok(self.take_expected_connection(endpoint).unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_expected_connection_is_what_connect_to_returns_from_either_end() {
        let addr: SocketAddr = "127.0.0.1:4242".parse().unwrap();
        let mut near_end = MockTransport::default();
        let far_end = near_end.try_get_remote_handle();

        // registered on one end of the pair, opened from the other
        let mut remote = far_end.expect_connection(addr);
        let mut local = near_end.connect_to(addr).await.unwrap();

        local.write_all_and_flush(b"hi").await.unwrap();
        assert_eq!(remote.read_n_bytes(2).await.unwrap(), b"hi");

        // one registration, one connection
        assert!(near_end.take_expected_connection(addr).is_none());
    }
}
