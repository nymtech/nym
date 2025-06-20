use crate::mixnet::{MixnetClient, MixnetClientSender, Recipient};
use crate::Error;
use futures::SinkExt;
use nym_sphinx::receiver::ReconstructedMessage;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex;
use tracing::field::debug;
use tracing::{debug, info, warn};

/**
 * TODO
 * - make bytes on the interface instead of InputMessage - always want Anonymous enum or?
 * - codec + check what sort of backpressure freaks everything out
 * - check tls works
 * - https://github.com/nymtech/nym-vpn-client/tree/develop/nym-vpn-core/crates/nym-ip-packet-client/src - hook into IPR
 * - builder pattern via MixSocket
 */

/// MixSocket is following the structure of something like Tokio::net::TcpSocket with regards to setup and interface, breakdown from TcpSocket to TcpStream, etc.
/// However, we can't map this one to one onto the TcpSocket as there isn't really a concept of binding to a port with the MixnetClient; it connects to its Gateway and then just accepts incoming messages from the Gw via the Websocket connection.
/// The cause for a MixSocket > going striaght to a MixStream is creating a Nym Client disconnected from the Mixnet first, then upgrading to a Stream when connecting it.
pub struct MixSocket {
    inner: MixnetClient,
}

impl MixSocket {
    /// Create a new socket that is disconnected from the Mixnet - kick off the Mixnet client with config for builder.
    /// Following idea of having single client with multiple concurrent connections represented by per-Recipient MixStream
    pub async fn new() -> Result<Self, Error> {
        todo!()
    }

    /// Connect to a specific peer (Nym Client) and return a Stream (cf TcpSocket::connect() / TcpStream::new())
    pub async fn connect_to(_recipient: Recipient) -> Result<MixStream, Error> {
        todo!()
    }

    /// Get our Nym address
    pub fn nym_address(&self) -> &Recipient {
        self.inner.nym_address()
    }

    pub fn get_ref(&self) -> &MixnetClient {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut MixnetClient {
        &mut self.inner
    }

    pub fn into_inner(self) -> MixnetClient {
        self.inner
    }
}

pub struct MixStream {
    client: MixnetClient,
    peer: Recipient,
}

impl MixStream {
    /// Create a MixStream and immediately connect (convenience method) or pass in a MixSocket (pre-configured DisconnectedMixnetClient)
    // TODO in future take config from MixSocket if exists in Option<> param, else spin up ephemeral client. Just doing ephemeral for initial sketch
    pub async fn new(_socket: Option<MixSocket>, peer: Recipient) -> Self {
        Self {
            client: MixnetClient::connect_new().await.unwrap(),
            peer,
        }
    }

    /// Nym address of Stream's peer
    pub fn peer_addr(&self) -> &Recipient {
        &self.peer
    }

    /// Our Nym address
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
    }

    /// TODO longer explanation TL;DR = following TcpStream's split into reader/writer for concurrency
    // Split for concurrent read/write (like TcpStream::Split)
    pub fn split(self) -> (MixStreamReader, MixStreamWriter) {
        let sender = self.client.split_sender();
        (
            MixStreamReader {
                client: self.client,
                peer: self.peer,
            },
            MixStreamWriter {
                sender,
                peer: self.peer,
            },
        )
    }

    /// Disconnect from the Mixnet
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
    }
}

impl AsyncRead for MixStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_read(cx, buf)
    }
}

impl AsyncWrite for MixStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.client).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_shutdown(cx)
    }
}

// TODO
pub struct MixStreamReader {
    client: MixnetClient,
    peer: Recipient,
}
impl MixStreamReader {}
// impl AsyncRead for MixStreamWriter {}

pub struct MixStreamWriter {
    sender: MixnetClientSender,
    peer: Recipient,
}
impl MixStreamWriter {}
// impl AsyncWrite for MixStreamWriter {}

/**
 * Tests TODO:
 * - simple setup + create MixStream & read/write to self
 * - "" & read/write to other
 * - make sure we can do TLS through this (aka get around the 'superinsecuredontuseinprod mode' flags)
 * - check we can push large amounts through and we dont lose anything - make a version of the simple test with codec + framing and try whack massive amounts through
 */

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Quick test fn for easy testing of sending to self
    impl MixSocket {
        pub async fn new_test() -> Result<Self, Error> {
            let inner = MixnetClient::connect_new().await?;
            Ok(MixSocket { inner })
        }
    }
}
