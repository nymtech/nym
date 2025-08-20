// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{leak, spawn_timeboxed};
use std::future::{Future, IntoFuture};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::error::Elapsed;

// a helper trait for use in tests to easily convert `T` into `&'static mut T`
pub trait Leak {
    fn leak(self) -> &'static mut Self;
}

impl<T> Leak for T {
    fn leak(self) -> &'static mut T {
        leak(self)
    }
}

// those are internal testing traits so we're not concerned about auto traits
#[allow(async_fn_in_trait)]
pub trait Timeboxed: IntoFuture + Sized {
    async fn timeboxed(self) -> Result<Self::Output, Elapsed> {
        self.execute_with_deadline(Duration::from_millis(200)).await
    }

    async fn execute_with_deadline(self, timeout: Duration) -> Result<Self::Output, Elapsed> {
        tokio::time::timeout(timeout, self).await
    }
}

impl<T> Timeboxed for T where T: IntoFuture + Sized {}

// those are internal testing traits so we're not concerned about auto traits
#[allow(async_fn_in_trait)]
pub trait Spawnable: Future + Sized + Send + 'static {
    fn spawn(self) -> JoinHandle<Self::Output>
    where
        <Self as Future>::Output: Send + 'static,
    {
        tokio::spawn(self)
    }
}

impl<T> Spawnable for T where T: Future + Sized + Send + 'static {}

pub trait TimeboxedSpawnable: Timeboxed + Spawnable {
    fn spawn_timeboxed(self) -> JoinHandle<Result<<Self as Future>::Output, Elapsed>>
    where
        <Self as Future>::Output: Send,
    {
        spawn_timeboxed(self)
    }
}

impl<T> TimeboxedSpawnable for T where T: Spawnable + Future + Send {}
