// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "io-mocks")]
use nym_test_utils::mocks::async_read_write::MockIOStream;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransport: AsyncRead + AsyncWrite + Sized {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self>;

    fn set_no_delay(&mut self, nodelay: bool) -> std::io::Result<()>;
}

impl LpTransport for TcpStream {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self> {
        TcpStream::connect(endpoint).await
    }

    fn set_no_delay(&mut self, nodelay: bool) -> std::io::Result<()> {
        // Set TCP_NODELAY for low latency
        self.set_nodelay(nodelay)
    }
}

#[cfg(feature = "io-mocks")]
impl LpTransport for MockIOStream {
    async fn connect(_endpoint: SocketAddr) -> std::io::Result<Self> {
        Ok(MockIOStream::default())
    }

    fn set_no_delay(&mut self, _nodelay: bool) -> std::io::Result<()> {
        Ok(())
    }
}
