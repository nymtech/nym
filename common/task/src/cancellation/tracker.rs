// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cancellation::token::ShutdownToken;
use crate::spawn::{spawn_named_future, JoinHandle};
use crate::spawn_future;
use std::future::Future;
use thiserror::Error;
use tokio_util::task::TaskTracker;
use tracing::{debug, trace};

#[derive(Debug, Error)]
#[error("task got cancelled")]
pub struct Cancelled;

/// Extracted [`TaskTracker`] and [`ShutdownToken`] to more easily allow tracking nested tasks
/// without having to pass whole [`ShutdownManager`] around
#[derive(Clone, Default, Debug)]
pub struct ShutdownTracker {
    pub(crate) root_cancellation_token: ShutdownToken,

    // the reason I'm not using a `JoinSet` is because it forces us to use futures with the same `::Output` type
    pub(crate) tracker: TaskTracker,
}

#[cfg(not(target_arch = "wasm32"))]
impl ShutdownTracker {
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_future(tracked)
    }

    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        trace!("attempting to spawn task {name}");
        let tracked = self.tracker.track_future(task);
        spawn_named_future(tracked, name)
    }

    #[track_caller]
    pub fn spawn_on<F>(&self, task: F, handle: &tokio::runtime::Handle) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn_on(task, handle)
    }

    #[track_caller]
    pub fn spawn_local<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.tracker.spawn_local(task)
    }

    #[track_caller]
    pub fn spawn_blocking<F, T>(&self, task: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.tracker.spawn_blocking(task)
    }

    #[track_caller]
    pub fn spawn_blocking_on<F, T>(&self, task: F, handle: &tokio::runtime::Handle) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.tracker.spawn_blocking_on(task, handle)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn try_spawn_named_with_shutdown<F>(
        &self,
        task: F,
        name: &str,
    ) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        trace!("attempting to spawn task {name} (with top-level cancellation)");

        let caller = std::panic::Location::caller();
        let shutdown_token = self.clone_shutdown_token();
        let name_owned = name.to_string();
        let tracked = self.tracker.track_future(async move {
            match shutdown_token.run_until_cancelled_owned(task).await {
                Some(result) => {
                    debug!("{name_owned} @ {caller}: task has finished execution");
                    Ok(result)
                }
                None => {
                    debug!("{name_owned} @ {caller}: shutdown signal received, shutting down");
                    Err(Cancelled)
                }
            }
        });
        spawn_named_future(tracked, name)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let caller = std::panic::Location::caller();
        let shutdown_token = self.clone_shutdown_token();
        self.tracker.spawn(async move {
            match shutdown_token.run_until_cancelled_owned(task).await {
                Some(result) => {
                    debug!("{caller}: task has finished execution");
                    Ok(result)
                }
                None => {
                    trace!("{caller}: shutdown signal received, shutting down");
                    Err(Cancelled)
                }
            }
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl ShutdownTracker {
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_future(tracked)
    }

    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_named_future(tracked, name)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn try_spawn_named_with_shutdown<F>(
        &self,
        task: F,
        name: &str,
    ) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let caller = std::panic::Location::caller();
        let shutdown_token = self.clone_shutdown_token();
        let tracked = self.tracker.track_future(async move {
            match shutdown_token.run_until_cancelled_owned(task).await {
                Some(result) => {
                    debug!("{caller}: task has finished execution");
                    Ok(result)
                }
                None => {
                    trace!("{caller}: shutdown signal received, shutting down");
                    Err(Cancelled)
                }
            }
        });
        spawn_named_future(tracked, name)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let caller = std::panic::Location::caller();
        let shutdown_token = self.clone_shutdown_token();
        let tracked = self.tracker.track_future(async move {
            match shutdown_token.run_until_cancelled_owned(task).await {
                Some(result) => {
                    debug!("{caller}: task has finished execution");
                    Ok(result)
                }
                None => {
                    trace!("{caller}: shutdown signal received, shutting down");
                    Err(Cancelled)
                }
            }
        });
        spawn_future(tracked)
    }
}

impl ShutdownTracker {
    pub fn child_shutdown_token(&self) -> ShutdownToken {
        self.root_cancellation_token.child_token()
    }

    pub fn clone_shutdown_token(&self) -> ShutdownToken {
        self.root_cancellation_token.clone()
    }
}
