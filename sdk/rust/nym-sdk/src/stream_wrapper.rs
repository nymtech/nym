use crate::mixnet::{InputMessage, MixnetClient, MixnetClientSender, Recipient};
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
 * - make bytes on the interface
 * - codec + check what sort of backpressure freaks everything out
 * - check tls works
 * - https://github.com/nymtech/nym-vpn-client/tree/develop/nym-vpn-core/crates/nym-ip-packet-client/src - hook into IPR
 */

/// Socket-like wrapper for reading and writing bytes over the Mixnet.
pub struct MixnetSocket {
    inner: MixnetClient,
}

impl MixnetSocket {
    pub fn new(client: MixnetClient) -> Self {
        Self { inner: client }
    }

    pub async fn connect() -> Result<Self, Error> {
        let client = MixnetClient::connect_new().await?;
        Ok(Self::new(client))
    }

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

    /// Split the socket into separate reader and writer halves - like TcpSockets.
    /// Wrap in Arc<Mutex<>> for concurrent read/write.
    pub fn split(self) -> (MixnetSocketReader, MixnetSocketWriter) {
        let client = Arc::new(Mutex::new(self.inner));
        let sender = {
            let mut client_guard = client.try_lock().expect("Failed to lock client for split");
            client_guard.split_sender()
        };
        (
            MixnetSocketReader {
                client: Arc::clone(&client),
            },
            MixnetSocketWriter { inner: sender },
        )
    }

    /// Disconnect from the Mixnet
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.inner.disconnect().await;
        debug!("Disconnected");
    }

    /// Wait for incoming messages from the Mixnet
    /// Returns None if the connection is closed, otherwise returns Some(Vec) of messages.
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        self.inner.wait_for_messages().await
    }

    /// Process incoming messages with a callback function - basically just moved this fn up a level
    pub async fn on_messages<F>(&mut self, fun: F)
    where
        F: Fn(ReconstructedMessage),
    {
        self.inner.on_messages(fun).await
    }

    // TODO change to bytes as input, remove the need for importing/knowing about InputMessage type & do conversion below this level
    pub async fn send(&mut self, message: InputMessage) -> Result<(), Error> {
        self.inner.send(message).await
    }
}

impl AsyncRead for MixnetSocket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for MixnetSocket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Read half of a split MixnetSocket
pub struct MixnetSocketReader {
    client: Arc<Mutex<MixnetClient>>,
}

impl AsyncRead for MixnetSocketReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.client.try_lock() {
            Ok(mut client) => {
                debug!("Got lock");
                Pin::new(&mut *client).poll_read(cx, buf)
            }
            Err(_) => {
                warn!("Couldn't immediately get lock");
                // If we can't get the lock immediately, wake the task and return Pending
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl MixnetSocketReader {
    /// Wait for messages from Mixnet
    /// Returns None if connection is closed.
    pub async fn wait_for_messages(&self) -> Option<Vec<ReconstructedMessage>> {
        let mut client = self.client.lock().await;
        client.wait_for_messages().await
    }

    pub async fn on_messages<F>(&self, fun: F)
    where
        F: Fn(ReconstructedMessage),
    {
        let mut client = self.client.lock().await;
        client.on_messages(fun).await
    }

    pub async fn nym_address(&self) -> Recipient {
        let client = self.client.lock().await;
        client.nym_address().clone()
    }
}

/// Write half of a split MixnetSocket
pub struct MixnetSocketWriter {
    inner: MixnetClientSender,
}

impl AsyncWrite for MixnetSocketWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl MixnetSocketWriter {
    pub async fn send(&mut self, message: InputMessage) -> Result<(), Error> {
        self.inner.send(message).await
    }
}

// Tests TODO:
// - make sure we can do TLS through this (aka get around the 'superinsecuredontuseinprod mode' flags)
// - check we can push large amounts through and we dont lose anything - make a version of the simple test with codec + framing and try whack massive amounts through
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet::MixnetClientBuilder;
    use log::info;
    use tokio::io::AsyncReadExt;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn simple_string_communication() -> Result<(), Error> {
        // nym_bin_common::logging::setup_logging();

        let client1 = MixnetClientBuilder::new_ephemeral()
            .build()?
            .connect_to_mixnet()
            .await?;

        let client2 = MixnetClientBuilder::new_ephemeral()
            .build()?
            .connect_to_mixnet()
            .await?;

        let client1_address = client1.nym_address().clone();

        let mut socket1 = MixnetSocket::new(client1);
        let mut socket2 = MixnetSocket::new(client2);

        let test_message = "Hello, Mixnet Socket!";

        let reader_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; 1024];
            let n = socket1.read(&mut buffer).await?;
            let received = String::from_utf8_lossy(&buffer[..n]).to_string();
            info!("Received: {}", received);
            Ok::<_, std::io::Error>(received)
        });

        // Wait for reader task to start
        tokio::time::sleep(Duration::from_secs(1)).await;

        // TODO need Inputmessage to be from_bytes - want just bytes on interface
        let input_message = InputMessage::simple(test_message.as_bytes(), client1_address);
        socket2.send(input_message).await?;

        let result = timeout(Duration::from_secs(30), reader_task).await;

        // Clean up
        socket2.disconnect().await;

        match result {
            Ok(Ok(received)) => {
                let unwrapped = received.unwrap();
                assert!(
                    unwrapped.contains(test_message),
                    "Split socket didn't receive the correct message - received {:?}",
                    unwrapped
                );
                Ok(())
            }
            Ok(Err(e)) => {
                panic!("Reader task failed with IO error: {}", e);
            }
            Err(_) => {
                panic!("Test timed out - failed to receive the message within 30 seconds");
            }
        }
    }

    #[tokio::test]
    async fn split_socket_communication() -> Result<(), Error> {
        // nym_bin_common::logging::setup_logging();

        let client1 = MixnetClientBuilder::new_ephemeral()
            .build()?
            .connect_to_mixnet()
            .await?;

        let client2 = MixnetClientBuilder::new_ephemeral()
            .build()?
            .connect_to_mixnet()
            .await?;

        let client1_address = client1.nym_address().clone();

        let socket1 = MixnetSocket::new(client1);
        let (mut reader, _writer) = socket1.split();

        let mut socket2 = MixnetSocket::new(client2);

        let test_message = "Hello from split socket!";

        let reader_task = tokio::spawn(async move {
            let messages = reader.wait_for_messages().await;
            match messages {
                Some(msgs) => {
                    if let Some(first_msg) = msgs.first() {
                        let content = String::from_utf8_lossy(&first_msg.message);
                        info!("Received via split socket: {}", content);
                        Ok(content.to_string())
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "No messages received",
                        ))
                    }
                }
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Connection closed",
                )),
            }
        });

        tokio::time::sleep(Duration::from_secs(1)).await;

        let input_message = InputMessage::simple(test_message.as_bytes(), client1_address);
        socket2.send(input_message).await?;

        let result = timeout(Duration::from_secs(30), reader_task).await;
        socket2.disconnect().await;

        match result {
            Ok(Ok(received)) => {
                let unwrapped = received.unwrap();
                assert!(
                    unwrapped.contains(test_message),
                    "Split socket didn't receive the correct message - received {:?}",
                    unwrapped
                );
                Ok(())
            }
            Ok(Err(e)) => {
                panic!("Split socket reader failed: {}", e);
            }
            Err(e) => {
                panic!("Split socket test failed: {}", e);
            }
        }
    }
}
