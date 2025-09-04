// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::event::SentStatus;
use std::future::Future;
use tokio_util::sync::{
    CancellationToken, DropGuard, WaitForCancellationFuture, WaitForCancellationFutureOwned,
};
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct ShutdownToken {
    inner: CancellationToken,
}

impl ShutdownToken {
    /// Leave the drop in no-op replacement for `send_status_msg` for easier migration from `TaskClient`.
    #[deprecated]
    #[track_caller]
    pub fn send_status_msg(&self, status: SentStatus) {
        let caller = std::panic::Location::caller();
        warn!("{caller} attempted to send {status} - there are no more listeners of those");
    }

    pub fn new() -> Self {
        ShutdownToken {
            inner: CancellationToken::new(),
        }
    }

    pub fn ephemeral() -> Self {
        ShutdownToken::default()
    }

    pub fn inner(&self) -> &CancellationToken {
        &self.inner
    }

    pub fn child_token(&self) -> ShutdownToken {
        ShutdownToken {
            inner: self.inner.child_token(),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }

    /// Returns `true` if the `ShutdownToken` is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.inner.cancelled()
    }

    pub fn cancelled_owned(self) -> WaitForCancellationFutureOwned {
        self.inner.cancelled_owned()
    }

    // Returned guard will cancel this token (and all its children) on drop unless disarmed.
    pub fn drop_guard(self) -> ShutdownDropGuard {
        ShutdownDropGuard {
            inner: self.inner.drop_guard(),
        }
    }

    pub async fn run_until_cancelled<F>(&self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.inner.run_until_cancelled(fut).await
    }

    pub async fn run_until_cancelled_owned<F>(self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.inner.run_until_cancelled_owned(fut).await
    }
}

pub struct ShutdownDropGuard {
    inner: DropGuard,
}

impl ShutdownDropGuard {
    pub fn disarm(self) -> ShutdownToken {
        ShutdownToken {
            inner: self.inner.disarm(),
        }
    }
}
