// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mocks::shared::InnerWrapper;
use futures::ready;
use std::fmt::{Display, Formatter};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::trace;

const INIT_ID: &str = "initialiser";
const RECV_ID: &str = "recipient";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Initialiser,
    Recipient,
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Initialiser => INIT_ID.fmt(f),
            Side::Recipient => RECV_ID.fmt(f),
        }
    }
}

// sending buffer of the first stream is the receiving buffer of the second stream
// and vice versa
pub fn mock_io_streams() -> (MockIOStream, MockIOStream) {
    let ch1 = MockIOStream::default();
    let ch2 = ch1.make_connection();

    (ch1, ch2)
}

pub struct MockIOStream {
    // identifier to use for logging purposes
    id: Arc<AtomicU8>,

    // side of the stream to use for logging purposes
    side: Side,

    // messages to send
    tx: InnerWrapper<Vec<u8>>,

    // messages to receive
    rx: InnerWrapper<Vec<u8>>,
}

impl Default for MockIOStream {
    fn default() -> Self {
        MockIOStream {
            id: Arc::new(AtomicU8::new(0)),
            side: Side::Initialiser,
            tx: Default::default(),
            rx: Default::default(),
        }
    }
}

impl MockIOStream {
    #[allow(clippy::panic)]
    fn make_connection(&self) -> Self {
        if self.side != Side::Initialiser {
            panic!("attempted to make invalid connection")
        }
        MockIOStream {
            id: self.id.clone(),
            side: Side::Recipient,
            tx: self.rx.cloned_buffer(),
            rx: self.tx.cloned_buffer(),
        }
    }

    pub fn set_id(&self, id: u8) {
        self.id.store(id, Ordering::Relaxed)
    }

    // the prefix `try_` is due to the fact that if the mock is cloned at an invalid state,
    // `assert!` will fail causing panic (which is fine in **test** code)
    pub fn try_get_remote_handle(&self) -> Self {
        self.make_connection()
    }

    // unwrap in test code is fine
    #[allow(clippy::unwrap_used)]
    pub fn unchecked_tx_data(&self) -> Vec<u8> {
        self.tx.buffer.try_lock().unwrap().content.clone()
    }

    // unwrap in test code is fine
    #[allow(clippy::unwrap_used)]
    pub fn unchecked_rx_data(&self) -> Vec<u8> {
        self.rx.buffer.try_lock().unwrap().content.clone()
    }

    fn log_read(&self, bytes: usize) {
        let id = self.id.load(Ordering::Relaxed);
        if id == 0 {
            trace!("[{}] read {bytes} bytes from mock stream", self.side)
        } else {
            trace!("[{}-{id}] read {bytes} bytes from mock stream", self.side)
        }
    }

    fn log_write(&self, bytes: usize) {
        let id = self.id.load(Ordering::Relaxed);

        if id == 0 {
            trace!("[{}] wrote {bytes} bytes to mock stream", self.side)
        } else {
            trace!("[{}-{id}] wrote {bytes} bytes to mock stream", self.side)
        }
    }
}

impl AsyncRead for MockIOStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        ready!(Pin::new(&mut self.rx).poll_guard_ready(cx));

        let unfilled = buf.remaining();

        // SAFETY: guard is ready
        #[allow(clippy::unwrap_used)]
        let guard = self.rx.guard().unwrap();

        let data = guard.take_at_most(unfilled);
        if data.is_empty() {
            // nothing to retrieve - store the waiter so that the sender could trigger it
            guard.waker = Some(cx.waker().clone());

            // drop the guard so that the sender could actually put messages in
            self.rx.transition_to_idle();
            return Poll::Pending;
        }

        self.log_read(data.len());
        // if let Some(waker) = guard.waker.take() {
        //     waker.wake();
        // }

        self.rx.transition_to_idle();

        buf.put_slice(&data);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockIOStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // wait until we transition to the locked state
        ready!(Pin::new(&mut self.tx).poll_guard_ready(cx));

        // SAFETY: guard is ready
        #[allow(clippy::unwrap_used)]
        let guard = self.tx.guard().unwrap();

        let len = buf.len();
        guard.content.extend_from_slice(buf);

        // TODO: if we wanted the behaviour of always reading everything before writing anything extra
        // if !guard.content.is_empty() {
        //     // sanity check
        //     assert!(guard.waker.is_none());
        //     guard.waker = Some(cx.waker().clone());
        //     self.tx.transition_to_idle();
        //     return Poll::Pending;
        // }

        self.log_write(buf.len());

        Poll::Ready(Ok(len))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Some(guard) = self.tx.guard() else {
            return Poll::Ready(Err(io::Error::other(
                "invalid lock state to send/flush messages",
            )));
        };

        if let Some(waker) = guard.waker.take() {
            // notify the receiver if it was waiting for messages
            waker.wake();
        }

        // release the guard
        self.tx.transition_to_idle();

        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // make sure our guard is always dropped on close
        self.tx.transition_to_idle();

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn basic() {
        let (mut stream1, mut stream2) = mock_io_streams();
        stream1.write_all(&[1, 2, 3, 4, 5]).await.unwrap();
        stream1.flush().await.unwrap();

        let mut buf = [0u8; 5];
        let read = stream2.read(&mut buf).await.unwrap();
        assert_eq!(read, 5);
        assert_eq!(&buf[0..5], &[1, 2, 3, 4, 5]);

        let mut buf = [0u8; 5];
        stream2.write_all(&[6, 7, 8, 9, 10]).await.unwrap();
        stream2.flush().await.unwrap();

        let read = stream1.read(&mut buf).await.unwrap();
        assert_eq!(read, 5);
        assert_eq!(&buf[0..5], &[6, 7, 8, 9, 10]);
    }
}
