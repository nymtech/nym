// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! TLS stream abstraction for plain and encrypted connections.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper_util::client::legacy::connect::{Connected, Connection};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pin_project! {
    /// A stream that may or may not be wrapped in TLS.
    #[project = MaybeTlsProj]
    pub enum MaybeTlsStream {
        Plain { #[pin] inner: tokio_smoltcp::TcpStream },
        Tls { #[pin] inner: smolmix_tls::TlsStream<tokio_smoltcp::TcpStream> },
    }
}

impl AsyncRead for MaybeTlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Plain { inner } => inner.poll_read(cx, buf),
            MaybeTlsProj::Tls { inner } => inner.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MaybeTlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.project() {
            MaybeTlsProj::Plain { inner } => inner.poll_write(cx, buf),
            MaybeTlsProj::Tls { inner } => inner.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Plain { inner } => inner.poll_flush(cx),
            MaybeTlsProj::Tls { inner } => inner.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Plain { inner } => inner.poll_shutdown(cx),
            MaybeTlsProj::Tls { inner } => inner.poll_shutdown(cx),
        }
    }
}

impl Connection for MaybeTlsStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}
