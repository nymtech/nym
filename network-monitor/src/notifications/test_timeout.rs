// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{delay_for, Delay, Duration, Instant};

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

    /// Forces self to fire regardless of internal Delay state
    pub(super) fn fire(&mut self) {
        match self.delay.as_mut() {
            None => error!("Tried to fire non-existent delay!"),
            // just set the next delay to 0 so it will be polled immediately and be already elapsed
            Some(delay) => delay.reset(Instant::now()),
        }
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
