// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::event::SentStatus;
use std::future::Future;
use tokio_util::sync::{
    CancellationToken, DropGuard, WaitForCancellationFuture, WaitForCancellationFutureOwned,
};
use tracing::warn;

/// A wrapped [CancellationToken](tokio_util::sync::CancellationToken) that is used for
/// signalling and listening for cancellation requests.
// We don't use CancellationToken in case we wanted to include additional fields/methods
// down the line.
#[derive(Debug, Clone, Default)]
pub struct ShutdownToken {
    inner: CancellationToken,
}

impl From<CancellationToken> for ShutdownToken {
    fn from(inner: CancellationToken) -> Self {
        ShutdownToken { inner }
    }
}

impl ShutdownToken {
    /// A drop in no-op replacement for `send_status_msg` for easier migration from [TaskClient](crate::TaskClient).
    #[deprecated]
    #[track_caller]
    pub fn send_status_msg(&self, status: SentStatus) {
        let caller = std::panic::Location::caller();
        warn!("{caller} attempted to send {status} - there are no more listeners of those");
    }

    /// Creates a new ShutdownToken in the non-cancelled state.
    pub fn new() -> Self {
        ShutdownToken {
            inner: CancellationToken::new(),
        }
    }

    /// Creates a new ShutdownToken given a tokio `CancellationToken`.
    pub fn new_from_tokio_token(cancellation_token: CancellationToken) -> Self {
        ShutdownToken {
            inner: cancellation_token,
        }
    }

    /// Gets reference to the underlying [CancellationToken](tokio_util::sync::CancellationToken).
    pub fn inner(&self) -> &CancellationToken {
        &self.inner
    }

    /// Get an owned [CancellationToken](tokio_util::sync::CancellationToken) for public API use.
    /// This is useful when you need to expose cancellation to SDK users without
    /// exposing the internal ShutdownToken type.
    pub fn to_cancellation_token(&self) -> CancellationToken {
        self.inner.clone()
    }

    /// Creates a `ShutdownToken` which will get cancelled whenever the
    /// current token gets cancelled. Unlike a cloned `ShutdownToken`,
    /// cancelling a child token does not cancel the parent token.
    ///
    /// If the current token is already cancelled, the child token will get
    /// returned in cancelled state.
    pub fn child_token(&self) -> ShutdownToken {
        ShutdownToken {
            inner: self.inner.child_token(),
        }
    }

    /// Cancel the underlying [CancellationToken](tokio_util::sync::CancellationToken) and all child tokens which had been
    /// derived from it.
    ///
    /// This will wake up all tasks which are waiting for cancellation.
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    /// Returns `true` if the underlying [CancellationToken](tokio_util::sync::CancellationToken) is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Returns a `Future` that gets fulfilled when cancellation is requested.
    ///
    /// The future will complete immediately if the token is already cancelled
    /// when this method is called.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe.
    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.inner.cancelled()
    }

    /// Returns a `Future` that gets fulfilled when cancellation is requested.
    ///
    /// The future will complete immediately if the token is already cancelled
    /// when this method is called.
    ///
    /// The function takes self by value and returns a future that owns the
    /// token.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe.
    pub fn cancelled_owned(self) -> WaitForCancellationFutureOwned {
        self.inner.cancelled_owned()
    }

    /// Creates a `ShutdownDropGuard` for this token.
    ///
    /// Returned guard will cancel this token (and all its children) on drop
    /// unless disarmed.
    pub fn drop_guard(self) -> ShutdownDropGuard {
        ShutdownDropGuard {
            inner: self.inner.drop_guard(),
        }
    }

    /// Runs a future to completion and returns its result wrapped inside an `Option`
    /// unless the `ShutdownToken` is cancelled. In that case the function returns
    /// `None` and the future gets dropped.
    ///
    /// # Cancel safety
    ///
    /// This method is only cancel safe if `fut` is cancel safe.
    pub async fn run_until_cancelled<F>(&self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.inner.run_until_cancelled(fut).await
    }

    /// Runs a future to completion and returns its result wrapped inside an `Option`
    /// unless the `ShutdownToken` is cancelled. In that case the function returns
    /// `None` and the future gets dropped.
    ///
    /// The function takes self by value and returns a future that owns the token.
    ///
    /// # Cancel safety
    ///
    /// This method is only cancel safe if `fut` is cancel safe.
    pub async fn run_until_cancelled_owned<F>(self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.inner.run_until_cancelled_owned(fut).await
    }
}

/// A wrapper for [DropGuard](tokio_util::sync::DropGuard) that wraps around a cancellation token
/// which automatically cancels it on drop.
/// It is created using `drop_guard` method on the `ShutdownToken`.
pub struct ShutdownDropGuard {
    inner: DropGuard,
}

impl ShutdownDropGuard {
    /// Returns stored [ShutdownToken](ShutdownToken) and removes this drop guard instance
    /// (i.e. it will no longer cancel token). Other guards for this token
    /// are not affected.
    pub fn disarm(self) -> ShutdownToken {
        ShutdownToken {
            inner: self.inner.disarm(),
        }
    }
}
