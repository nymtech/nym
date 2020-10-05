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

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{delay_for, Delay, Duration};

pub(super) struct TestTimeout {
    delay: Option<Delay>,
}

impl TestTimeout {
    pub(super) fn new() -> Self {
        TestTimeout { delay: None }
    }

    pub(super) fn start(&mut self, duration: Duration) {
        self.delay = Some(delay_for(duration))
    }

    pub(super) fn clear(&mut self) {
        self.delay = None
    }
}

impl Future for TestTimeout {
    type Output = <Delay as Future>::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.delay.as_mut() {
            None => Poll::Pending,
            Some(delay) => Pin::new(delay).poll(cx),
        }
    }
}
