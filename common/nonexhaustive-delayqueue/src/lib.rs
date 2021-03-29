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

use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::time::Instant;
use tokio_stream::Stream;
use tokio_util::time::{delay_queue, DelayQueue};

pub use tokio::time::error::Error as TimerError;
pub use tokio_util::time::delay_queue::Expired;
pub type QueueKey = delay_queue::Key;

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
