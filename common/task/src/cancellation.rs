// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::event::SentStatus;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::error;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::{
    CancellationToken, DropGuard, WaitForCancellationFuture, WaitForCancellationFutureOwned,
};
use tokio_util::task::TaskTracker;
use tracing::{debug, info, trace, warn};

use crate::spawn::{spawn_named_future, JoinHandle};
use crate::spawn_future;
use tokio::task::JoinSet;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

pub const DEFAULT_MAX_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
#[error("task got cancelled")]
pub struct Cancelled;

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

#[derive(Default)]
pub struct ShutdownSignals(JoinSet<()>);

impl ShutdownSignals {
    pub async fn wait_for_signal(&mut self) {
        self.0.join_next().await;
    }
}

/// Extracted [`TaskTracker`] and [`ShutdownToken`] to more easily allow tracking nested tasks
/// without having to pass whole [`ShutdownManager`] around
#[derive(Clone, Default, Debug)]
pub struct ShutdownTracker {
    root_cancellation_token: ShutdownToken,

    // the reason I'm not using a `JoinSet` is because it forces us to use futures with the same `::Output` type
    tracker: TaskTracker,
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

#[allow(deprecated)]
pub struct ShutdownManager {
    legacy_task_manager: Option<crate::TaskManager>,

    shutdown_signals: ShutdownSignals,

    tracker: ShutdownTracker,

    max_shutdown_duration: Duration,
}

// note: default implementation will ONLY listen for SIGINT and will ignore SIGTERM and SIGQUIT
// this is due to result type when registering the signal
#[cfg(not(target_arch = "wasm32"))]
impl Default for ShutdownManager {
    fn default() -> Self {
        ShutdownManager::new_without_signals()
            .with_interrupt_signal()
            .with_cancel_on_panic()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ShutdownManager {
    #[must_use]
    #[track_caller]
    pub fn with_shutdown<F>(mut self, shutdown: F) -> Self
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        let shutdown_token = self.tracker.clone_shutdown_token();
        self.shutdown_signals.0.spawn(async move {
            shutdown.await;

            info!("sending cancellation after receiving shutdown signal");
            shutdown_token.cancel();
        });
        self
    }

    #[allow(deprecated)]
    pub fn with_legacy_task_manager(mut self) -> Self {
        let mut legacy_manager = crate::TaskManager::default().named("legacy-task-manager");
        let mut legacy_error_rx = legacy_manager.task_return_error_rx();
        let mut legacy_drop_rx = legacy_manager.task_drop_rx();

        self.legacy_task_manager = Some(legacy_manager);

        // add a task that listens for legacy task clients being dropped to trigger cancellation
        self.with_shutdown(async move {
            tokio::select! {
                _ = legacy_error_rx.recv() => (),
                _ = legacy_drop_rx.recv() => (),
            }

            info!("received legacy shutdown signal");
        })
    }

    #[cfg(unix)]
    #[track_caller]
    pub fn with_shutdown_signal(self, signal_kind: SignalKind) -> std::io::Result<Self> {
        let mut sig = signal(signal_kind)?;
        Ok(self.with_shutdown(async move {
            sig.recv().await;
        }))
    }

    #[cfg(unix)]
    #[track_caller]
    pub fn with_terminate_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::terminate())
    }

    #[cfg(unix)]
    #[track_caller]
    pub fn with_quit_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::quit())
    }

    pub fn with_default_shutdown_signals(self) -> std::io::Result<Self> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                self.with_interrupt_signal()
                    .with_terminate_signal()?
                    .with_quit_signal()
            } else {
                Ok(self.with_interrupt_signal())
            }
        }
    }

    #[track_caller]
    pub fn with_interrupt_signal(self) -> Self {
        self.with_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
        })
    }

    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(task)
    }

    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.try_spawn_named(task, name)
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
        self.tracker.try_spawn_named_with_shutdown(task, name)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn_with_shutdown(task)
    }
}

#[cfg(target_arch = "wasm32")]
impl ShutdownManager {
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.tracker.spawn(task)
    }

    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.tracker.try_spawn_named(task, name)
    }

    #[track_caller]
    pub fn try_spawn_named_with_shutdown<F>(
        &self,
        task: F,
        name: &str,
    ) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.try_spawn_named_with_shutdown(task, name)
    }

    /// Spawn the task that will get cancelled if a global shutdown signal is detected
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.spawn_with_shutdown(task)
    }
}

impl ShutdownManager {
    /// Create new instance of `ShutdownManager` without any shutdown signals registered,
    /// meaning it will only attempt to for all tasks spawned on its tracker to gracefully finish execution.
    pub fn new_without_signals() -> Self {
        let manager = ShutdownManager {
            legacy_task_manager: None,
            shutdown_signals: Default::default(),
            tracker: Default::default(),
            max_shutdown_duration: Duration::from_secs(10),
        };

        // we need to add an explicit watcher for the cancellation token being cancelled
        // so that we could cancel all legacy tasks
        cfg_if::cfg_if! {if #[cfg(not(target_arch = "wasm32"))] {
            let cancel_watcher = manager.tracker.clone_shutdown_token();
            manager.with_shutdown(async move { cancel_watcher.cancelled().await })
        } else {
            manager
        }}
    }

    pub fn empty_mock() -> Self {
        ShutdownManager {
            legacy_task_manager: None,
            shutdown_signals: Default::default(),
            tracker: Default::default(),
            max_shutdown_duration: Default::default(),
        }
    }

    #[must_use]
    pub fn with_cancel_on_panic(self) -> Self {
        let current_hook = std::panic::take_hook();

        let shutdown_token = self.clone_shutdown_token();
        std::panic::set_hook(Box::new(move |panic_info| {
            // 1. call existing hook
            current_hook(panic_info);

            let location = panic_info
                .location()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());

            let payload = if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
                payload
            } else {
                ""
            };

            // 2. issue cancellation
            error!("panicked at {location}: {payload}. issuing global cancellation");
            shutdown_token.cancel();
        }));
        self
    }

    #[must_use]
    pub fn with_shutdown_duration(mut self, duration: Duration) -> Self {
        self.max_shutdown_duration = duration;
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.tracker.root_cancellation_token.is_cancelled()
    }

    pub fn shutdown_tracker(&self) -> &ShutdownTracker {
        &self.tracker
    }

    pub fn shutdown_tracker_owned(&self) -> ShutdownTracker {
        self.tracker.clone()
    }

    pub async fn wait_for_tracker(&self) {
        self.tracker.tracker.wait().await;
    }

    pub fn close_tracker(&self) -> bool {
        self.tracker.tracker.close()
    }

    pub fn reopen_tracker(&self) -> bool {
        self.tracker.tracker.reopen()
    }

    pub fn is_tracker_closed(&self) -> bool {
        self.tracker.tracker.is_closed()
    }

    pub fn is_tracker_empty(&self) -> bool {
        self.tracker.tracker.is_empty()
    }

    pub fn tracked_tasks(&self) -> usize {
        self.tracker.tracker.len()
    }

    pub fn child_shutdown_token(&self) -> ShutdownToken {
        self.tracker.root_cancellation_token.child_token()
    }

    pub fn clone_shutdown_token(&self) -> ShutdownToken {
        self.tracker.root_cancellation_token.clone()
    }

    #[must_use]
    #[allow(deprecated)]
    pub fn subscribe_legacy<S: Into<String>>(&self, child_suffix: S) -> crate::TaskClient {
        // alternatively we could have set self.legacy_task_manager = Some(TaskManager::default());
        // on demand if it wasn't unavailable, but then we'd have to use mutable reference
        #[allow(clippy::expect_used)]
        self.legacy_task_manager
            .as_ref()
            .expect("did not enable legacy shutdown support")
            .subscribe_named(child_suffix)
    }

    async fn finish_shutdown(mut self) {
        let mut wait_futures = FuturesUnordered::<Pin<Box<dyn Future<Output = ()> + Send>>>::new();

        // force shutdown via ctrl-c
        wait_futures.push(Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            let interrupt_future = tokio::signal::ctrl_c();

            #[cfg(target_arch = "wasm32")]
            let interrupt_future = futures::future::pending::<()>();

            let _ = interrupt_future.await;
            info!("received interrupt - forcing shutdown");
        }));

        // timeout
        wait_futures.push(Box::pin(async move {
            sleep(self.max_shutdown_duration).await;
            info!("timeout reached - forcing shutdown");
        }));

        // graceful
        wait_futures.push(Box::pin(async move {
            self.wait_for_tracker().await;
            info!("all tracked tasks successfully shutdown");
            if let Some(legacy) = self.legacy_task_manager.as_mut() {
                legacy.wait_for_graceful_shutdown().await;
                info!("all legacy tasks successfully shutdown");
            }

            info!("all registered tasks successfully shutdown")
        }));

        wait_futures.next().await;
    }

    pub fn detach_shutdown_signals(&mut self) -> ShutdownSignals {
        mem::take(&mut self.shutdown_signals)
    }

    pub fn replace_shutdown_signals(&mut self, signals: ShutdownSignals) {
        self.shutdown_signals = signals;
    }

    pub fn send_cancellation(&self) {
        if let Some(legacy_manager) = self.legacy_task_manager.as_ref() {
            info!("attempting to shutdown legacy tasks");
            let _ = legacy_manager.signal_shutdown();
        }
        self.tracker.root_cancellation_token.cancel();
    }

    /// Wait until receiving one of the registered shutdown signals
    /// this method is cancellation safe
    pub async fn wait_for_shutdown_signal(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.shutdown_signals.0.join_next().await;

        #[cfg(target_arch = "wasm32")]
        self.tracker.root_cancellation_token.cancelled().await;
    }

    /// Perform system shutdown by sending relevant signals and waiting until either:
    /// - all tracked tasks have terminated
    /// - timeout has been reached
    /// - shutdown has been forced (by sending SIGINT)
    pub async fn perform_shutdown(self) {
        self.send_cancellation();

        info!("waiting for tasks to finish... (press ctrl-c to force)");
        self.finish_shutdown().await;
    }

    /// Wait until a shutdown signal has been received and trigger system shutdown.
    pub async fn run_until_shutdown(mut self) {
        self.close_tracker();
        self.wait_for_shutdown_signal().await;

        self.perform_shutdown().await;
    }
}
