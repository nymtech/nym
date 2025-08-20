// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mocks::shared::{ContentWrapper, InnerWrapper};
use anyhow::{anyhow, bail};
use futures::{ready, Sink, Stream};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

// sending buffer of the first stream is the receiving buffer of the second stream
// and vice versa
pub fn mock_streams<T>() -> (MockStream<T>, MockStream<T>)
where
    T: Send,
{
    let ch1 = MockStream::default();
    let ch2 = ch1.make_connection();

    (ch1, ch2)
}

pub struct MockStream<T: 'static> {
    // messages to send
    tx: InnerWrapper<VecDeque<T>>,

    // messages to receive
    rx: InnerWrapper<VecDeque<T>>,
}

impl<T> MockStream<T> {
    pub fn clone_tx_buffer(&self) -> Arc<Mutex<ContentWrapper<VecDeque<T>>>>
    where
        T: Send,
    {
        self.tx.clone_buffer()
    }

    pub fn clone_rx_buffer(&self) -> Arc<Mutex<ContentWrapper<VecDeque<T>>>>
    where
        T: Send,
    {
        self.rx.clone_buffer()
    }

    fn make_connection(&self) -> Self
    where
        T: Send,
    {
        MockStream {
            tx: self.rx.cloned_buffer(),
            rx: self.tx.cloned_buffer(),
        }
    }
}

impl<T> Default for MockStream<T> {
    fn default() -> Self {
        MockStream {
            tx: InnerWrapper::default(),
            rx: InnerWrapper::default(),
        }
    }
}

impl<T> Stream for MockStream<T>
where
    T: Send,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        ready!(Pin::new(&mut self.rx).poll_guard_ready(cx));

        // SAFETY: guard is ready
        #[allow(clippy::unwrap_used)]
        let guard = self.rx.guard().unwrap();

        let Some(next) = guard.content.pop_front() else {
            // nothing to retrieve - store the waiter so that the sender could trigger it
            guard.waker = Some(cx.waker().clone());

            // drop the guard so that the sender could actually put messages in
            self.rx.transition_to_idle();
            return Poll::Pending;
        };

        // there are more messages buffered waiting for us to retrieve
        // keep the guard!
        if !guard.content.is_empty() {
            cx.waker().wake_by_ref();
        } else {
            // no more messages, drop the guard
            self.rx.transition_to_idle();
        }

        Poll::Ready(Some(next))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // that's just a minor optimisation, so don't sweat about it too much,
        // if we can obtain the mutex, give precise information, otherwise return default values
        let Ok(guard) = self.rx.buffer.try_lock() else {
            return (0, None);
        };
        let items = guard.content.len();
        (items, Some(items))
    }
}

impl<T> Sink<T> for MockStream<T>
where
    T: Send,
{
    type Error = anyhow::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // wait until we transition to the locked state
        ready!(Pin::new(&mut self.tx).poll_guard_ready(cx));
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let Some(guard) = self.tx.guard() else {
            bail!("invalid lock state to send messages");
        };
        guard.content.push_back(item);

        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let Some(guard) = self.tx.guard() else {
            return Poll::Ready(Err(anyhow!("invalid lock state to send/flush messages")));
        };

        if let Some(waker) = guard.waker.take() {
            // notify the receiver if it was waiting for messages
            waker.wake();
        }

        // release the guard
        self.tx.transition_to_idle();

        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        // make sure our guard is always dropped on close
        self.tx.transition_to_idle();

        Poll::Ready(Ok(()))
    }
}
