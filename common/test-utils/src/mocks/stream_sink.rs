// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use futures::future::BoxFuture;
use futures::{ready, FutureExt, Sink, Stream};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::sync::{Mutex, OwnedMutexGuard};

// sending buffer of the first stream is the receiving buffer of the second stream
// and vice versa
pub fn mock_streams<T>() -> (MockStream<T>, MockStream<T>) {
    let ch1 = MockStream::default();
    let ch2 = ch1.make_connection();

    (ch1, ch2)
}

pub struct MockStream<T: 'static> {
    // messages to send
    tx: MockStreamInner<T>,

    // messages to receive
    rx: MockStreamInner<T>,
}

struct MockStreamInner<T: 'static> {
    buffer: Arc<Mutex<MessagesWrapper<T>>>,
    lock_state: StreamLockState<T>,
}

#[derive(Default)]
enum StreamLockState<T> {
    // We haven’t started locking yet
    #[default]
    Idle,

    // Waiting for the mutex lock future to resolve
    TryingToLock(BoxFuture<'static, OwnedMutexGuard<MessagesWrapper<T>>>),

    // We hold the mutex guard
    Locked(OwnedMutexGuard<MessagesWrapper<T>>),
}

impl<T> MockStream<T> {
    pub fn clone_tx_buffer(&self) -> Arc<Mutex<MessagesWrapper<T>>> {
        self.tx.buffer.clone()
    }

    pub fn clone_rx_buffer(&self) -> Arc<Mutex<MessagesWrapper<T>>> {
        self.rx.buffer.clone()
    }

    fn make_connection(&self) -> Self {
        MockStream {
            tx: MockStreamInner {
                buffer: self.rx.buffer.clone(),
                lock_state: StreamLockState::Idle,
            },
            rx: MockStreamInner {
                buffer: self.tx.buffer.clone(),
                lock_state: StreamLockState::Idle,
            },
        }
    }
}

impl<T> Default for MockStream<T> {
    fn default() -> Self {
        MockStream {
            tx: MockStreamInner {
                buffer: Arc::new(Mutex::new(MessagesWrapper::default())),
                lock_state: StreamLockState::Idle,
            },
            rx: MockStreamInner {
                buffer: Arc::new(Mutex::new(MessagesWrapper::default())),
                lock_state: StreamLockState::Idle,
            },
        }
    }
}

pub struct MessagesWrapper<T> {
    messages: VecDeque<T>,
    waker: Option<Waker>,
}

impl<T> Default for MessagesWrapper<T> {
    fn default() -> Self {
        MessagesWrapper {
            messages: VecDeque::new(),
            waker: None,
        }
    }
}

impl<T> Stream for MockStream<T>
where
    T: Send,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.rx.lock_state {
            StreamLockState::Idle => {
                // 1. first try to obtain the guard without locking
                let Ok(guard) = self.rx.buffer.clone().try_lock_owned() else {
                    // 2. if that fails, create the future for obtaining it
                    self.rx.lock_state =
                        StreamLockState::TryingToLock(self.rx.buffer.clone().lock_owned().boxed());
                    return Poll::Pending;
                };

                // correctly transition to locked state and poll ourselves again
                self.rx.lock_state = StreamLockState::Locked(guard);
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            StreamLockState::TryingToLock(lock_fut) => {
                // see if the guard future has resolved, if so, transition to locked state and schedule for another poll
                let guard = ready!(lock_fut.as_mut().poll(cx));
                self.rx.lock_state = StreamLockState::Locked(guard);
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            StreamLockState::Locked(guard) => {
                let Some(next) = guard.messages.pop_front() else {
                    // nothing to retrieve - store the waiter so that the sender could trigger it
                    guard.waker = Some(cx.waker().clone());

                    // drop the guard so that the sender could actually put messages in
                    self.rx.lock_state = StreamLockState::Idle;
                    return Poll::Pending;
                };

                // there are more messages buffered waiting for us to retrieve
                // keep the guard!
                if !guard.messages.is_empty() {
                    cx.waker().wake_by_ref();
                } else {
                    // no more messages, drop the guard
                    self.rx.lock_state = StreamLockState::Idle
                }

                Poll::Ready(Some(next))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // that's just a minor optimisation, so don't sweat about it too much,
        // if we can obtain the mutex, give precise information, otherwise return default values
        let Ok(guard) = self.rx.buffer.try_lock() else {
            return (0, None);
        };
        let items = guard.messages.len();
        (items, Some(items))
    }
}

impl<T> Sink<T> for MockStream<T>
where
    T: Send,
{
    type Error = anyhow::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.tx.lock_state {
            StreamLockState::Idle => {
                // 1. first try to obtain the guard without locking
                if let Ok(guard) = self.tx.buffer.clone().try_lock_owned() {
                    self.tx.lock_state = StreamLockState::Locked(guard);
                    return Poll::Ready(Ok(()));
                }

                // 2. if that fails, create the future for obtaining it
                self.tx.lock_state =
                    StreamLockState::TryingToLock(self.tx.buffer.clone().lock_owned().boxed());
                // schedule ourselves for polling again so that we would be controlled by the lock future
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            StreamLockState::TryingToLock(lock_fut) => {
                // see if the guard future has resolved
                let guard = ready!(lock_fut.as_mut().poll(cx));
                self.tx.lock_state = StreamLockState::Locked(guard);
                Poll::Ready(Ok(()))
            }
            StreamLockState::Locked(_) => {
                // if we have the guard, we're ready
                Poll::Ready(Ok(()))
            }
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        match &mut self.tx.lock_state {
            StreamLockState::Locked(guard) => {
                guard.messages.push_back(item);
            }
            _ => bail!("invalid lock state to send messages"),
        }

        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        match &mut self.tx.lock_state {
            StreamLockState::Locked(guard) => {
                if let Some(waker) = guard.waker.take() {
                    // notify the receiver if it was waiting for messages
                    waker.wake();
                }
            }
            _ => return Poll::Ready(Err(anyhow!("invalid lock state to send/flush messages"))),
        }
        // release the guard
        self.tx.lock_state = StreamLockState::Idle;

        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        // make sure our guard is always dropped on close
        self.tx.lock_state = StreamLockState::Idle;
        Poll::Ready(Ok(()))
    }
}
