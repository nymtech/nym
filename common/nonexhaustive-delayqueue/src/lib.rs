// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio_stream::Stream;

// this is a copy of tokio-util delay_queue with `Sleep` and `Instant` being replaced with
// `wasm_timer` equivalents

#[cfg(not(target_arch = "wasm32"))]
type DelayQueue<T> = tokio_util::time::DelayQueue<T>;
#[cfg(not(target_arch = "wasm32"))]
pub use tokio_util::time::delay_queue::Expired;
#[cfg(not(target_arch = "wasm32"))]
pub type QueueKey = tokio_util::time::delay_queue::Key;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;

#[cfg(target_arch = "wasm32")]
type DelayQueue<T> = wasmtimer::tokio_util::DelayQueue<T>;
#[cfg(target_arch = "wasm32")]
pub use wasmtimer::tokio_util::delay_queue::Expired;
#[cfg(target_arch = "wasm32")]
pub type QueueKey = wasmtimer::tokio_util::delay_queue::Key;
#[cfg(target_arch = "wasm32")]
use wasmtimer::std::Instant;

/// A variant of tokio's `DelayQueue`, such that its `Stream` implementation will never return a 'None'.
pub struct NonExhaustiveDelayQueue<T> {
    inner: DelayQueue<T>,
    waker: Option<Waker>,
}

// more methods of underlying DelayQueue will get exposed as we need them
impl<T> NonExhaustiveDelayQueue<T> {
    pub fn new() -> Self {
        NonExhaustiveDelayQueue {
            inner: DelayQueue::new(),
            waker: None,
        }
    }

    pub fn insert(&mut self, value: T, timeout: Duration) -> QueueKey {
        let key = self.inner.insert(value, timeout);
        if let Some(waker) = self.waker.take() {
            // we were waiting for an item - wake the executor!
            waker.wake()
        }
        key
    }

    pub fn insert_at(&mut self, value: T, when: Instant) -> QueueKey {
        let key = self.inner.insert_at(value, when);
        if let Some(waker) = self.waker.take() {
            // we were waiting for an item - wake the executor!
            waker.wake()
        }
        key
    }

    // TODO: it seems like this one can cause panic in very rare edge cases, however,
    // I can't seem to be able to reproduce it at all.
    pub fn remove(&mut self, key: &QueueKey) -> Expired<T> {
        self.inner.remove(key)
    }
}

impl<T> Default for NonExhaustiveDelayQueue<T> {
    fn default() -> Self {
        NonExhaustiveDelayQueue::new()
    }
}

impl<T> Stream for NonExhaustiveDelayQueue<T> {
    type Item = <DelayQueue<T> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) => {
                // we'll need to keep the waker to notify the executor once we get new item
                self.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//
// }
