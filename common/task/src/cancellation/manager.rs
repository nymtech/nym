// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cancellation::tracker::{Cancelled, ShutdownTracker};
use crate::spawn::JoinHandle;
use crate::ShutdownToken;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::error;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::time::Duration;
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::sleep;

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinSet;

/// A top level structure responsible for controlling process shutdown by listening to
/// the underlying registered signals and issuing cancellation to tasks derived from its root cancellation token.
#[allow(deprecated)]
pub struct ShutdownManager {
    /// Optional reference to the legacy [TaskManager](crate::TaskManager) to allow easier
    /// transition to the new system.
    pub(crate) legacy_task_manager: Option<crate::TaskManager>,

    /// Registered [ShutdownSignals](ShutdownSignals) that will trigger process shutdown if detected.
    pub(crate) shutdown_signals: ShutdownSignals,

    /// Combined [TaskTracker](tokio_util::task::TaskTracker) and [ShutdownToken](ShutdownToken)
    /// for spawning and tracking tasks associated with this ShutdownManager.
    pub(crate) tracker: ShutdownTracker,

    /// The maximum shutdown duration when tracked tasks could gracefully exit
    /// before forcing the shutdown.
    pub(crate) max_shutdown_duration: Duration,
}

/// Wrapper behind futures that upon completion will trigger binary shutdown.
#[derive(Default)]
pub struct ShutdownSignals(JoinSet<()>);

impl ShutdownSignals {
    /// Wait for any of the registered signals to be ready
    pub async fn wait_for_signal(&mut self) {
        self.0.join_next().await;
    }
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
    /// Create new instance of ShutdownManager with the most sensible defaults, so that:
    /// - shutdown will be triggered upon either SIGINT, SIGTERM (unix only) or SIGQUIT (unix only)  being sent
    /// - shutdown will be triggered upon any task panicking
    pub fn build_new_default() -> std::io::Result<Self> {
        Ok(ShutdownManager::new_without_signals()
            .with_default_shutdown_signals()?
            .with_cancel_on_panic())
    }

    /// Register a new shutdown signal that upon completion will trigger system shutdown.
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

    /// Include support for the legacy [TaskManager](TaskManager) to this instance of the ShutdownManager.
    /// This will allow issuing [TaskClient](TaskClient) for tasks that still require them.
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

    /// Add the specified signal to the currently registered shutdown signals that will trigger
    /// cancellation of all registered tasks.
    #[cfg(unix)]
    #[track_caller]
    pub fn with_shutdown_signal(self, signal_kind: SignalKind) -> std::io::Result<Self> {
        let mut sig = signal(signal_kind)?;
        Ok(self.with_shutdown(async move {
            sig.recv().await;
        }))
    }

    /// Add the SIGTERM signal to the currently registered shutdown signals that will trigger
    /// cancellation of all registered tasks.
    #[cfg(unix)]
    #[track_caller]
    pub fn with_terminate_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::terminate())
    }

    /// Add the SIGQUIT signal to the currently registered shutdown signals that will trigger
    /// cancellation of all registered tasks.
    #[cfg(unix)]
    #[track_caller]
    pub fn with_quit_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::quit())
    }

    /// Add default signals to the set of the currently registered shutdown signals that will trigger
    /// cancellation of all registered tasks.
    /// This includes SIGINT, SIGTERM and SIGQUIT for unix-based platforms and SIGINT for other targets (such as windows)/
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

    /// Add the SIGINT (ctrl-c) signal to the currently registered shutdown signals that will trigger
    /// cancellation of all registered tasks.
    #[track_caller]
    pub fn with_interrupt_signal(self) -> Self {
        self.with_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
        })
    }

    /// Spawn the provided future on the current Tokio runtime, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tracker.spawn(task)
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
        self.tracker.try_spawn_named(task, name)
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
        self.tracker.try_spawn_named_with_shutdown(task, name)
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
        self.tracker.spawn_with_shutdown(task)
    }
}

#[cfg(target_arch = "wasm32")]
impl ShutdownManager {
    /// Run the provided future on the current thread, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.tracker.spawn(task)
    }

    /// Run the provided future on the current thread, and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    /// It has exactly the same behaviour as [spawn](Self::spawn) and it only exists to provide
    /// the same interface as non-wasm32 targets.
    #[track_caller]
    pub fn try_spawn_named<F>(&self, task: F, name: &str) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        self.tracker.try_spawn_named(task, name)
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
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.try_spawn_named_with_shutdown(task, name)
    }

    /// Run the provided future on the current thread
    /// that will get cancelled once a global shutdown signal is detected,
    /// and track it in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    #[track_caller]
    pub fn spawn_with_shutdown<F>(&self, task: F) -> JoinHandle<Result<F::Output, Cancelled>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tracker.spawn_with_shutdown(task)
    }
}

impl ShutdownManager {
    /// Create new instance of ShutdownManager without any external shutdown signals registered,
    /// meaning it will only attempt to wait for all tasks spawned on its tracker to gracefully finish execution.
    pub fn new_without_signals() -> Self {
        Self::new_from_external_shutdown_token(ShutdownToken::new())
    }

    /// Create new instance of the ShutdownManager using an external shutdown token.
    ///
    /// Note: it will not listen to any external shutdown signals!
    /// You might want further customise it with [shutdown signals](Self::with_shutdown)
    /// (or just use [the default set](Self::with_default_shutdown_signals).
    /// Similarly, you might want to include [cancellation on panic](Self::with_cancel_on_panic)
    /// to make sure everything gets cancelled if one of the tasks panics.
    pub fn new_from_external_shutdown_token(shutdown_token: ShutdownToken) -> Self {
        let manager = ShutdownManager {
            legacy_task_manager: None,
            shutdown_signals: Default::default(),
            tracker: ShutdownTracker::new_from_external_shutdown_token(shutdown_token),
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

    /// Create an empty testing mock of the ShutdownManager with no signals registered.
    pub fn empty_mock() -> Self {
        ShutdownManager {
            legacy_task_manager: None,
            shutdown_signals: Default::default(),
            tracker: Default::default(),
            max_shutdown_duration: Default::default(),
        }
    }

    /// Add additional panic hook such that upon triggering, the root [ShutdownToken](ShutdownToken) gets cancelled.
    /// Note: an unfortunate limitation of this is that graceful shutdown will no longer be possible
    /// since that task that has panicked will not exit and thus all shutdowns will have to be either forced
    /// or will have to time out.
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

    /// Change the maximum shutdown duration when tracked tasks could gracefully exit
    /// before forcing the shutdown.
    #[must_use]
    pub fn with_shutdown_duration(mut self, duration: Duration) -> Self {
        self.max_shutdown_duration = duration;
        self
    }

    /// Returns true if the root [ShutdownToken](ShutdownToken) has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.tracker.root_cancellation_token.is_cancelled()
    }

    /// Get a reference to the used [ShutdownTracker](ShutdownTracker)
    pub fn shutdown_tracker(&self) -> &ShutdownTracker {
        &self.tracker
    }

    /// Get a cloned instance of the used [ShutdownTracker](ShutdownTracker)
    pub fn shutdown_tracker_owned(&self) -> ShutdownTracker {
        self.tracker.clone()
    }

    /// Waits until the underlying [TaskTracker](tokio_util::task::TaskTracker) is both closed and empty.
    ///
    /// If the underlying [TaskTracker](tokio_util::task::TaskTracker) is already closed and empty when this method is called, then it
    /// returns immediately.
    pub async fn wait_for_tracker(&self) {
        self.tracker.wait_for_tracker().await;
    }

    /// Close the underlying [TaskTracker](tokio_util::task::TaskTracker).
    ///
    /// This allows [`wait_for_tracker`] futures to complete. It does not prevent you from spawning new tasks.
    ///
    /// Returns `true` if this closed the underlying [TaskTracker](tokio_util::task::TaskTracker), or `false` if it was already closed.
    ///
    /// [`wait_for_tracker`]: ShutdownTracker::wait_for_tracker
    pub fn close_tracker(&self) -> bool {
        self.tracker.close_tracker()
    }

    /// Reopen the underlying [TaskTracker](tokio_util::task::TaskTracker).
    ///
    /// This prevents [`wait_for_tracker`] futures from completing even if the underlying [TaskTracker](tokio_util::task::TaskTracker) is empty.
    ///
    /// Returns `true` if this reopened the underlying [TaskTracker](tokio_util::task::TaskTracker), or `false` if it was already open.
    ///
    /// [`wait_for_tracker`]: ShutdownTracker::wait_for_tracker
    pub fn reopen_tracker(&self) -> bool {
        self.tracker.reopen_tracker()
    }

    /// Returns `true` if the underlying [TaskTracker](tokio_util::task::TaskTracker) is [closed](Self::close_tracker).
    pub fn is_tracker_closed(&self) -> bool {
        self.tracker.is_tracker_closed()
    }

    /// Returns the number of tasks tracked by the underlying [TaskTracker](tokio_util::task::TaskTracker).
    pub fn tracked_tasks(&self) -> usize {
        self.tracker.tracked_tasks()
    }

    /// Returns `true` if there are no tasks in the underlying [TaskTracker](tokio_util::task::TaskTracker).
    pub fn is_tracker_empty(&self) -> bool {
        self.tracker.is_tracker_empty()
    }

    /// Obtain a [ShutdownToken](crate::cancellation::ShutdownToken) that is a child of the root token
    pub fn child_shutdown_token(&self) -> ShutdownToken {
        self.tracker.root_cancellation_token.child_token()
    }

    /// Obtain a [ShutdownToken](crate::cancellation::ShutdownToken) on the same hierarchical structure as the root token
    pub fn clone_shutdown_token(&self) -> ShutdownToken {
        self.tracker.root_cancellation_token.clone()
    }

    /// Attempt to create a handle to a legacy [TaskClient] to support tasks that hasn't migrated
    /// from the legacy [TaskManager].
    /// Note. To use this method [ShutdownManager] must be built with `.with_legacy_task_manager()`
    #[must_use]
    #[deprecated]
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

    /// Finalise the shutdown procedure by waiting until either:
    /// - all tracked tasks have terminated
    /// - timeout has been reached
    /// - shutdown has been forced (by sending SIGINT)
    async fn finish_shutdown(&mut self) {
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
        let max_shutdown = self.max_shutdown_duration;
        wait_futures.push(Box::pin(async move {
            sleep(max_shutdown).await;
            info!("timeout reached - forcing shutdown");
        }));

        // graceful
        let tracker = self.tracker.clone();
        wait_futures.push(Box::pin(async move {
            tracker.wait_for_tracker().await;
            info!("all tracked tasks successfully shutdown");
            if let Some(legacy) = self.legacy_task_manager.as_mut() {
                legacy.wait_for_graceful_shutdown().await;
                info!("all legacy tasks successfully shutdown");
            }

            info!("all registered tasks successfully shutdown")
        }));

        wait_futures.next().await;
    }

    /// Remove the current set of [ShutdownSignals] from this instance of
    /// [ShutdownManager] replacing it with an empty set.
    ///
    /// This is potentially useful if one wishes to start listening for the signals
    /// before the whole process has been fully set up.
    pub fn detach_shutdown_signals(&mut self) -> ShutdownSignals {
        mem::take(&mut self.shutdown_signals)
    }

    /// Replace the current set of [ShutdownSignals] used for determining
    /// whether the underlying process should be stopped.
    pub fn replace_shutdown_signals(&mut self, signals: ShutdownSignals) {
        self.shutdown_signals = signals;
    }

    /// Send cancellation signal to all registered tasks by cancelling the root token
    /// and sending shutdown signal, if applicable, on the legacy [TaskManager]
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
    pub async fn perform_shutdown(&mut self) {
        self.send_cancellation();

        info!("waiting for tasks to finish... (press ctrl-c to force)");
        self.finish_shutdown().await;
    }

    /// Wait until a shutdown signal has been received and trigger system shutdown.
    pub async fn run_until_shutdown(&mut self) {
        self.close_tracker();
        self.wait_for_shutdown_signal().await;

        self.perform_shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::traits::{ElapsedExt, Timeboxed};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[tokio::test]
    async fn shutdown_with_no_tracked_tasks_and_signals() -> anyhow::Result<()> {
        let mut manager = ShutdownManager::new_without_signals();
        let res = manager.run_until_shutdown().timeboxed().await;
        assert!(res.has_elapsed());

        let mut manager = ShutdownManager::new_without_signals();
        let shutdown = manager.clone_shutdown_token();
        shutdown.cancel();
        let res = manager.run_until_shutdown().timeboxed().await;
        assert!(!res.has_elapsed());

        Ok(())
    }

    #[tokio::test]
    async fn shutdown_signal() -> anyhow::Result<()> {
        let timeout_shutdown = sleep(Duration::from_millis(100));
        let mut manager = ShutdownManager::new_without_signals().with_shutdown(timeout_shutdown);

        // execution finishes after the sleep gets finishes
        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;
        assert!(!res.has_elapsed());

        Ok(())
    }

    #[tokio::test]
    async fn panic_hook() -> anyhow::Result<()> {
        let mut manager = ShutdownManager::new_without_signals().with_cancel_on_panic();
        manager.spawn_with_shutdown(async move {
            sleep(Duration::from_millis(10000)).await;
        });
        manager.spawn_with_shutdown(async move {
            sleep(Duration::from_millis(10)).await;
            panic!("panicking");
        });

        // execution finishes after the panic gets triggered
        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;
        assert!(!res.has_elapsed());

        Ok(())
    }

    #[tokio::test]
    async fn task_cancellation() -> anyhow::Result<()> {
        let timeout_shutdown = sleep(Duration::from_millis(100));
        let mut manager = ShutdownManager::new_without_signals().with_shutdown(timeout_shutdown);

        let cancelled1 = Arc::new(AtomicBool::new(false));
        let cancelled1_clone = cancelled1.clone();
        let cancelled2 = Arc::new(AtomicBool::new(false));
        let cancelled2_clone = cancelled2.clone();

        let shutdown = manager.clone_shutdown_token();
        manager.spawn(async move {
            shutdown.cancelled().await;
            cancelled1_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let shutdown = manager.clone_shutdown_token();
        manager.spawn(async move {
            shutdown.cancelled().await;
            cancelled2_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;

        assert!(!res.has_elapsed());
        assert!(cancelled1.load(std::sync::atomic::Ordering::Relaxed));
        assert!(cancelled2.load(std::sync::atomic::Ordering::Relaxed));
        Ok(())
    }

    #[tokio::test]
    async fn cancellation_within_task() -> anyhow::Result<()> {
        let mut manager = ShutdownManager::new_without_signals();

        let cancelled1 = Arc::new(AtomicBool::new(false));
        let cancelled1_clone = cancelled1.clone();

        let shutdown = manager.clone_shutdown_token();
        manager.spawn(async move {
            shutdown.cancelled().await;
            cancelled1_clone.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let shutdown = manager.clone_shutdown_token();
        manager.spawn(async move {
            sleep(Duration::from_millis(10)).await;
            shutdown.cancel();
        });

        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;

        assert!(!res.has_elapsed());
        assert!(cancelled1.load(std::sync::atomic::Ordering::Relaxed));
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_timeout() -> anyhow::Result<()> {
        let timeout_shutdown = sleep(Duration::from_millis(50));
        let mut manager = ShutdownManager::new_without_signals()
            .with_shutdown(timeout_shutdown)
            .with_shutdown_duration(Duration::from_millis(1000));

        // ignore shutdown signals
        manager.spawn(async move {
            sleep(Duration::from_millis(1000)).await;
        });

        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;

        assert!(res.has_elapsed());

        let timeout_shutdown = sleep(Duration::from_millis(50));
        let mut manager = ShutdownManager::new_without_signals()
            .with_shutdown(timeout_shutdown)
            .with_shutdown_duration(Duration::from_millis(100));

        // ignore shutdown signals
        manager.spawn(async move {
            sleep(Duration::from_millis(1000)).await;
        });

        let res = manager
            .run_until_shutdown()
            .execute_with_deadline(Duration::from_millis(200))
            .await;

        assert!(!res.has_elapsed());
        Ok(())
    }
}
