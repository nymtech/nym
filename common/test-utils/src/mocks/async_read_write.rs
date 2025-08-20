// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mocks::shared::InnerWrapper;
use futures::ready;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// sending buffer of the first stream is the receiving buffer of the second stream
// and vice versa
pub fn mock_io_streams() -> (MockIOStream, MockIOStream) {
    let ch1 = MockIOStream::default();
    let ch2 = ch1.make_connection();

    (ch1, ch2)
}

#[derive(Default)]
pub struct MockIOStream {
    // messages to send
    tx: InnerWrapper<Vec<u8>>,

    // messages to receive
    rx: InnerWrapper<Vec<u8>>,
}

impl MockIOStream {
    fn make_connection(&self) -> Self {
        MockIOStream {
            tx: self.rx.cloned_buffer(),
            rx: self.tx.cloned_buffer(),
        }
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
}

impl AsyncRead for MockIOStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        ready!(Pin::new(&mut self.rx).poll_guard_ready(cx));

        // SAFETY: guard is ready
        #[allow(clippy::unwrap_used)]
        let guard = self.rx.guard().unwrap();

        let data = guard.take_content();
        if data.is_empty() {
            // nothing to retrieve - store the waiter so that the sender could trigger it
            guard.waker = Some(cx.waker().clone());

            // drop the guard so that the sender could actually put messages in
            self.rx.transition_to_idle();
            return Poll::Pending;
        }

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
