// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(crate) struct DummyDevice;

impl AsyncRead for DummyDevice {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        unimplemented!("tunnel devices are not supported by non-linux targets")
    }
}

impl AsyncWrite for DummyDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::result::Result<usize, Error>> {
        unimplemented!("tunnel devices are not supported by non-linux targets")
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        unimplemented!("tunnel devices are not supported by non-linux targets")
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Error>> {
        unimplemented!("tunnel devices are not supported by non-linux targets")
    }
}
