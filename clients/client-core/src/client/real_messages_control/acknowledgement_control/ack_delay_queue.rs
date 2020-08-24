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

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::time::{
    delay_queue::{self, Expired},
    DelayQueue,
};

// works under assumption that it will be used inside a loop, where we never want a `None`
// TODO: perhaps this should/could be renamed and moved to common/utils (and expose all inner methods?)
pub struct AckDelayQueue<T> {
    inner: DelayQueue<T>,
    waker: Option<Waker>,
}

// more methods of underlying DelayQueue will get exposed as we need them
impl<T> AckDelayQueue<T> {
    pub fn new() -> Self {
        AckDelayQueue {
            inner: DelayQueue::new(),
            waker: None,
        }
    }

    pub fn insert(&mut self, value: T, timeout: Duration) -> delay_queue::Key {
        let key = self.inner.insert(value, timeout);
        if let Some(waker) = self.waker.take() {
            // we were waiting for an item - wake the executor!
            waker.wake()
        }
        key
    }

    pub fn remove(&mut self, key: &delay_queue::Key) -> Expired<T> {
        self.inner.remove(key)
    }
}

impl<T> Stream for AckDelayQueue<T> {
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
