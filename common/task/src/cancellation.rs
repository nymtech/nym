// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{TaskClient, TaskManager};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tokio_util::sync::{CancellationToken, DropGuard};
use tokio_util::task::TaskTracker;
use tracing::{debug, info, trace};

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

pub const DEFAULT_MAX_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

pub fn token_name(name: &Option<String>) -> String {
    name.clone().unwrap_or_else(|| "unknown".to_string())
}

// a wrapper around tokio's CancellationToken that adds optional `name` information to more easily
// track down sources of shutdown
#[derive(Debug, Default)]
pub struct ShutdownToken {
    name: Option<String>,
    inner: CancellationToken,
}

impl Clone for ShutdownToken {
    fn clone(&self) -> Self {
        // make sure to not accidentally overflow the stack if we keep cloning the handle
        let name = if let Some(name) = &self.name {
            if name != Self::OVERFLOW_NAME && name.len() < Self::MAX_NAME_LENGTH {
                Some(format!("{name}-child"))
            } else {
                Some(Self::OVERFLOW_NAME.to_string())
            }
        } else {
            None
        };

        ShutdownToken {
            name,
            inner: self.inner.clone(),
        }
    }
}

impl Deref for ShutdownToken {
    type Target = CancellationToken;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ShutdownToken {
    const MAX_NAME_LENGTH: usize = 128;
    const OVERFLOW_NAME: &'static str = "reached maximum ShutdownToken children name depth";

    pub fn new(name: impl Into<String>) -> Self {
        ShutdownToken {
            name: Some(name.into()),
            inner: CancellationToken::new(),
        }
    }

    // Creates a ShutdownToken which will get cancelled whenever the current token gets cancelled.
    // Unlike a cloned/forked ShutdownToken, cancelling a child token does not cancel the parent token.
    #[must_use]
    pub fn child_token<S: Into<String>>(&self, child_suffix: S) -> Self {
        let suffix = child_suffix.into();
        let child_name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };

        ShutdownToken {
            name: Some(child_name),
            inner: self.inner.child_token(),
        }
    }

    // Creates a clone of the ShutdownToken which will get cancelled whenever the current token gets cancelled, and vice versa.
    #[must_use]
    pub fn clone_with_suffix<S: Into<String>>(&self, child_suffix: S) -> Self {
        let mut child = self.clone();
        let suffix = child_suffix.into();
        let child_name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };

        child.name = Some(child_name);
        child
    }

    // exposed method with the old name for easier migration
    // it will eventually be removed so please try to use `.clone_with_suffix` instead
    #[must_use]
    pub fn fork<S: Into<String>>(&self, child_suffix: S) -> Self {
        self.clone_with_suffix(child_suffix)
    }

    // exposed method with the old name for easier migration
    // it will eventually be removed so please try to use `.clone().named(name)` instead
    #[must_use]
    pub fn fork_named<S: Into<String>>(&self, name: S) -> Self {
        self.clone().named(name)
    }

    #[must_use]
    pub fn named<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn add_suffix<S: Into<String>>(self, suffix: S) -> Self {
        let suffix = suffix.into();
        let name = if let Some(base) = &self.name {
            format!("{base}-{suffix}")
        } else {
            format!("unknown-{suffix}")
        };
        self.named(name)
    }

    // Returned guard will cancel this token (and all its children) on drop unless disarmed.
    pub fn drop_guard(self) -> ShutdownDropGuard {
        ShutdownDropGuard {
            name: self.name,
            inner: self.inner.drop_guard(),
        }
    }

    pub fn name(&self) -> String {
        token_name(&self.name)
    }

    pub async fn run_until_cancelled<F>(&self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        let res = self.inner.run_until_cancelled(fut).await;
        trace!("'{}' got cancelled", self.name());
        res
    }
}

pub struct ShutdownDropGuard {
    name: Option<String>,
    inner: DropGuard,
}

impl Deref for ShutdownDropGuard {
    type Target = DropGuard;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ShutdownDropGuard {
    pub fn disarm(self) -> ShutdownToken {
        ShutdownToken {
            name: self.name,
            inner: self.inner.disarm(),
        }
    }

    pub fn name(&self) -> String {
        token_name(&self.name)
    }
}

pub struct ShutdownManager {
    pub root_token: ShutdownToken,

    legacy_task_manager: Option<TaskManager>,

    shutdown_signals: JoinSet<()>,

    // the reason I'm not using a `JoinSet` is because it forces us to use futures with the same `::Output` type
    tracker: TaskTracker,

    max_shutdown_duration: Duration,
}

impl Deref for ShutdownManager {
    type Target = TaskTracker;

    fn deref(&self) -> &Self::Target {
        &self.tracker
    }
}

impl ShutdownManager {
    pub fn new(root_token_name: impl Into<String>) -> Self {
        let manager = ShutdownManager {
            root_token: ShutdownToken::new(root_token_name),
            legacy_task_manager: None,
            shutdown_signals: Default::default(),
            tracker: Default::default(),
            max_shutdown_duration: Duration::from_secs(10),
        };

        // we need to add an explicit watcher for the cancellation token being cancelled
        // so that we could cancel all legacy tasks
        let cancel_watcher = manager.root_token.clone();
        manager.with_shutdown(async move { cancel_watcher.cancelled().await })
    }

    pub fn with_legacy_task_manager(mut self) -> Self {
        let mut legacy_manager =
            TaskManager::default().named(format!("{}-legacy", self.root_token.name()));
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

    #[cfg(not(target_arch = "wasm32"))]
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

    #[must_use]
    pub fn with_shutdown<F>(mut self, shutdown: F) -> Self
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        let shutdown_token = self.root_token.clone();
        self.shutdown_signals.spawn(async move {
            shutdown.await;

            info!("sending cancellation after receiving shutdown signal");
            shutdown_token.cancel();
        });
        self
    }

    #[cfg(unix)]
    pub fn with_shutdown_signal(self, signal_kind: SignalKind) -> std::io::Result<Self> {
        let mut sig = signal(signal_kind)?;
        Ok(self.with_shutdown(async move {
            sig.recv().await;
        }))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_interrupt_signal(self) -> Self {
        self.with_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
        })
    }

    #[cfg(unix)]
    pub fn with_terminate_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::terminate())
    }

    #[cfg(unix)]
    pub fn with_quit_signal(self) -> std::io::Result<Self> {
        self.with_shutdown_signal(SignalKind::quit())
    }

    #[must_use]
    pub fn with_shutdown_duration(mut self, duration: Duration) -> Self {
        self.max_shutdown_duration = duration;
        self
    }

    pub fn child_token<S: Into<String>>(&self, child_suffix: S) -> ShutdownToken {
        self.root_token.child_token(child_suffix)
    }

    pub fn clone_token<S: Into<String>>(&self, child_suffix: S) -> ShutdownToken {
        self.root_token.clone_with_suffix(child_suffix)
    }

    #[must_use]
    pub fn subscribe_legacy<S: Into<String>>(&self, child_suffix: S) -> TaskClient {
        // alternatively we could have set self.legacy_task_manager = Some(TaskManager::default());
        // on demand if it wasn't unavailable, but then we'd have to use mutable reference
        #[allow(clippy::expect_used)]
        self.legacy_task_manager
            .as_ref()
            .expect("did not enable legacy shutdown support")
            .subscribe_named(child_suffix)
    }

    async fn finish_shutdown(mut self) {
        let mut wait_futures = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();

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
            info!("timeout reached, forcing shutdown");
        }));

        // graceful
        wait_futures.push(Box::pin(async move {
            self.tracker.wait().await;
            debug!("migrated tasks successfully shutdown");
            if let Some(legacy) = self.legacy_task_manager.as_mut() {
                legacy.wait_for_graceful_shutdown().await;
                debug!("legacy tasks successfully shutdown");
            }

            info!("all registered tasks successfully shutdown")
        }));

        wait_futures.next().await;
    }

    pub async fn wait_for_shutdown_signal(mut self) {
        self.shutdown_signals.join_next().await;

        if let Some(legacy_manager) = self.legacy_task_manager.as_mut() {
            info!("attempting to shutdown legacy tasks");
            let _ = legacy_manager.signal_shutdown();
        }

        info!("waiting for tasks to finish... (press ctrl-c to force)");
        self.finish_shutdown().await;
    }
}
