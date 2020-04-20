// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::task::{Context, Poll};
use futures::AsyncWrite;
use std::io;
use std::pin::Pin;
use tokio::prelude::*;

pub(crate) struct ConnectionWriter {
    connection: tokio::net::TcpStream,
}

impl ConnectionWriter {
    pub(crate) fn new(connection: tokio::net::TcpStream) -> Self {
        ConnectionWriter { connection }
    }
}

impl AsyncWrite for ConnectionWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        use tokio::io::AsyncWrite;

        let mut read_buf = [0; 1];
        match Pin::new(&mut self.connection).poll_read(cx, &mut read_buf) {
            // at least try the obvious check for if connection is definitely down
            // TODO: can we do anything else?
            Poll::Ready(Ok(n)) if n == 0 => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "trying to write to closed connection",
            ))),
            _ => Pin::new(&mut self.connection).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use tokio::io::AsyncWrite;
        Pin::new(&mut self.connection).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use tokio::io::AsyncWrite;
        Pin::new(&mut self.connection).poll_shutdown(cx)
    }
}
