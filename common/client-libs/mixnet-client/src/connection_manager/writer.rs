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
use futures::Sink;
use nymsphinx::framing::codec::{SphinxCodec, SphinxCodecError};
use nymsphinx::framing::packet::FramedSphinxPacket;
use std::pin::Pin;
use tokio_util::codec::Framed;

pub(crate) struct ConnectionWriter {
    framed_connection: Framed<tokio::net::TcpStream, SphinxCodec>,
}

impl ConnectionWriter {
    pub(crate) fn new(connection: tokio::net::TcpStream) -> Self {
        ConnectionWriter {
            framed_connection: Framed::new(connection, SphinxCodec),
        }
    }
}

impl Sink<FramedSphinxPacket> for ConnectionWriter {
    type Error = SphinxCodecError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.framed_connection).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: FramedSphinxPacket) -> Result<(), Self::Error> {
        Pin::new(&mut self.framed_connection).start_send(item)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.framed_connection).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.framed_connection).poll_close(cx)
    }
}
