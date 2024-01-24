// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;

use pin_project::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::stream::NoiseStream;

#[pin_project(project = ConnectionProj)]
pub enum Connection {
    Tcp(#[pin] TcpStream),
    Noise(#[pin] NoiseStream),
}

impl Connection {
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, io::Error> {
        match self {
            Self::Noise(stream) => stream.peer_addr(),
            Self::Tcp(stream) => stream.peer_addr(),
        }
    }
}

impl AsyncRead for Connection {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_read(cx, buf),
            ConnectionProj::Tcp(stream) => stream.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_write(cx, buf),
            ConnectionProj::Tcp(stream) => stream.poll_write(cx, buf),
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_flush(cx),
            ConnectionProj::Tcp(stream) => stream.poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_shutdown(cx),
            ConnectionProj::Tcp(stream) => stream.poll_shutdown(cx),
        }
    }
}
