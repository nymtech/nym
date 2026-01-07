// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpConfig;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

#[cfg(feature = "io-mocks")]
use nym_test_utils::mocks::async_read_write::MockIOStream;

// only used in internal code (and tests)
#[allow(async_fn_in_trait)]
pub trait LpTransportLayer: AsyncRead + AsyncWrite + Sized {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self>;

    fn configure(&mut self, config: &LpConfig) -> std::io::Result<()>;
}

impl LpTransportLayer for TcpStream {
    async fn connect(endpoint: SocketAddr) -> std::io::Result<Self> {
        TcpStream::connect(endpoint).await
    }

    fn configure(&mut self, config: &LpConfig) -> std::io::Result<()> {
        // Set TCP_NODELAY for low latency
        self.set_nodelay(config.tcp_nodelay)
    }
}

#[cfg(feature = "io-mocks")]
impl LpTransportLayer for MockIOStream {
    async fn connect(_endpoint: SocketAddr) -> std::io::Result<Self> {
        Ok(MockIOStream::default())
    }

    fn configure(&mut self, _config: &LpConfig) -> std::io::Result<()> {
        Ok(())
    }
}
