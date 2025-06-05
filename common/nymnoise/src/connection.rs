// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;

use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::stream::NoiseStream;

//SW once plain TCP support is dropped, this whole enum can be dropped, and we can only propagate NoiseStream
#[pin_project(project = ConnectionProj)]
pub enum Connection<C> {
    Raw(#[pin] C),
    Noise(#[pin] Box<NoiseStream<C>>),
}

impl<C> AsyncRead for Connection<C>
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_read(cx, buf),
            ConnectionProj::Raw(stream) => stream.poll_read(cx, buf),
        }
    }
}

impl<C> AsyncWrite for Connection<C>
where
    C: AsyncWrite + AsyncRead + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_write(cx, buf),
            ConnectionProj::Raw(stream) => stream.poll_write(cx, buf),
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_flush(cx),
            ConnectionProj::Raw(stream) => stream.poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        match self.project() {
            ConnectionProj::Noise(stream) => stream.poll_shutdown(cx),
            ConnectionProj::Raw(stream) => stream.poll_shutdown(cx),
        }
    }
}
