// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cancellation::token::ShutdownToken;
use crate::spawn::{JoinHandle, spawn_named_future};
use crate::spawn_future;
use std::future::Future;
use thiserror::Error;
use tokio_util::task::TaskTracker;
use tracing::{debug, trace};

#[derive(Debug, Error)]
#[error("task got cancelled")]
pub struct Cancelled;

/// Extracted [TaskTracker](tokio_util::task::TaskTracker) and [ShutdownToken](ShutdownToken) to more easily allow tracking nested tasks
/// without having to pass whole [ShutdownManager](ShutdownManager) around.
#[derive(Clone, Default, Debug)]
pub struct ShutdownTracker {
    /// The root [ShutdownToken](ShutdownToken) that will trigger all derived tasks
    /// to receive cancellation signal.
    pub(crate) root_cancellation_token: ShutdownToken,

    // Note: the reason we're not using a `JoinSet` is
    // because it forces us to use futures with the same `::Output` type,
    // which is not really a desirable property in this instance.
    /// Tracker used for keeping track of all registered tasks
    /// so that they could be stopped gracefully before ending the process.
    pub(crate) tracker: TaskTracker,
}

#[cfg(not(target_arch = "wasm32"))]
impl ShutdownTracker {
    /// Spawn the provided future on the current Tokio runtime, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_future(tracked)
    }

    /// Spawn the provided future on the current Tokio runtime,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    /// Furthermore, attach a name to the spawned task to more easily track it within a [tokio console](https://github.com/tokio-rs/console)
    ///
    /// Note that is no different from [spawn](Self::spawn) if the underlying binary
    /// has not been built with `RUSTFLAGS="--cfg tokio_unstable"` and `--features="tokio-tracing"`
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

    /// Spawn the provided future on the provided Tokio runtime,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_on<F>(&self, task: F, handle: &tokio::runtime::Handle) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn_on(task, handle)
    }

    /// Spawn the provided future on the current [LocalSet](tokio::task::LocalSet),
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_local<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.tracker.spawn_local(task)
    }

    /// Spawn the provided blocking task on the current Tokio runtime,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_blocking<F, T>(&self, task: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.tracker.spawn_blocking(task)
    }

    /// Spawn the provided blocking task on the provided Tokio runtime,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_blocking_on<F, T>(&self, task: F, handle: &tokio::runtime::Handle) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.tracker.spawn_blocking_on(task, handle)
    }

    /// Spawn the provided future on the current Tokio runtime
    /// that will get cancelled once a global shutdown signal is detected,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    ///
    /// Note that to fully use the naming feature, such as tracking within a [tokio console](https://github.com/tokio-rs/console),
    /// the underlying binary has to be built with `RUSTFLAGS="--cfg tokio_unstable"` and `--features="tokio-tracing"`
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
                    trace!("{name_owned} @ {caller}: shutdown signal received, shutting down");
                    Err(Cancelled)
                }
            }
        });
        spawn_named_future(tracked, name)
    }

    /// Spawn the provided future on the current Tokio runtime
    /// that will get cancelled once a global shutdown signal is detected,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
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
    /// Run the provided future on the current thread, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_future(tracked)
    }

    /// Run the provided future on the current thread, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    /// It has exactly the same behaviour as [spawn](Self::spawn) and it only exists to provide
    /// the same interface as non-wasm32 targets.
    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        let tracked = self.tracker.track_future(task);
        spawn_named_future(tracked, name)
    }

    /// Run the provided future on the current thread
    /// that will get cancelled once a global shutdown signal is detected,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    /// It has exactly the same behaviour as [spawn_with_shutdown](Self::spawn_with_shutdown) and it only exists to provide
    /// the same interface as non-wasm32 targets.
    #[track_caller]
    pub fn try_spawn_named_with_shutdown<F>(
        &self,
        task: F,
        name: &str,
    ) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + 'static,
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

    /// Run the provided future on the current thread
    /// that will get cancelled once a global shutdown signal is detected,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + 'static,
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
    /// Create new instance of the ShutdownTracker using an external shutdown token.
    /// This could be useful in situations where shutdown is being managed by an external entity
    /// that is not [ShutdownManager](ShutdownManager), but interface requires providing a ShutdownTracker,
    /// such as client-core tasks
    pub fn new_from_external_shutdown_token(shutdown_token: ShutdownToken) -> Self {
        ShutdownTracker {
            root_cancellation_token: shutdown_token,
            tracker: Default::default(),
        }
    }

    /// Waits until the underlying [TaskTracker](tokio_util::task::TaskTracker) is both closed and empty.
    ///
    /// If the underlying [TaskTracker](tokio_util::task::TaskTracker) is already closed and empty when this method is called, then it
    /// returns immediately.
    pub async fn wait_for_tracker(&self) {
        self.tracker.wait().await;
    }

    /// Close the underlying [TaskTracker](tokio_util::task::TaskTracker).
    ///
    /// This allows [`wait_for_tracker`] futures to complete. It does not prevent you from spawning new tasks.
    ///
    /// Returns `true` if this closed the underlying [TaskTracker](tokio_util::task::TaskTracker), or `false` if it was already closed.
    ///
    /// [`wait_for_tracker`]: Self::wait_for_tracker
    pub fn close_tracker(&self) -> bool {
        self.tracker.close()
    }

    /// Reopen the underlying [TaskTracker](tokio_util::task::TaskTracker).
    ///
    /// This prevents [`wait_for_tracker`] futures from completing even if the underlying [TaskTracker](tokio_util::task::TaskTracker) is empty.
    ///
    /// Returns `true` if this reopened the underlying [TaskTracker](tokio_util::task::TaskTracker), or `false` if it was already open.
    ///
    /// [`wait_for_tracker`]: Self::wait_for_tracker
    pub fn reopen_tracker(&self) -> bool {
        self.tracker.reopen()
    }

    /// Returns `true` if the underlying [TaskTracker](tokio_util::task::TaskTracker) is [closed](Self::close_tracker).
    pub fn is_tracker_closed(&self) -> bool {
        self.tracker.is_closed()
    }

    /// Returns the number of tasks tracked by the underlying [TaskTracker](tokio_util::task::TaskTracker).
    pub fn tracked_tasks(&self) -> usize {
        self.tracker.len()
    }

    /// Returns `true` if there are no tasks in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    pub fn is_tracker_empty(&self) -> bool {
        self.tracker.is_empty()
    }

    /// Obtain a [ShutdownToken](crate::cancellation::ShutdownToken) that is a child of the root token
    pub fn child_shutdown_token(&self) -> ShutdownToken {
        self.root_cancellation_token.child_token()
    }

    /// Obtain a [ShutdownToken](crate::cancellation::ShutdownToken) on the same hierarchical structure as the root token
    pub fn clone_shutdown_token(&self) -> ShutdownToken {
        self.root_cancellation_token.clone()
    }

    /// Create a child ShutdownTracker that inherits cancellation from this tracker
    /// but has its own TaskTracker for managing sub-tasks.
    ///
    /// This enables hierarchical task management where:
    /// - Parent cancellation flows to all children
    /// - Each level tracks its own tasks independently
    /// - Components can wait for their specific sub-tasks to complete
    pub fn child_tracker(&self) -> ShutdownTracker {
        // Child token inherits cancellation from parent
        let child_token = self.root_cancellation_token.child_token();

        // New TaskTracker for this level's tasks
        let child_task_tracker = TaskTracker::new();

        ShutdownTracker {
            root_cancellation_token: child_token,
            tracker: child_task_tracker,
        }
    }

    /// Convenience method to perform a complete shutdown sequence.
    /// This method:
    /// 1. Signals cancellation to all tasks
    /// 2. Closes the tracker to prevent new tasks
    /// 3. Waits for all existing tasks to complete
    pub async fn shutdown(self) {
        // Signal cancellation to all tasks
        self.root_cancellation_token.cancel();

        // Close the tracker to prevent new tasks from being spawned
        self.tracker.close();

        // Wait for all existing tasks to complete
        self.tracker.wait().await;
    }
}
