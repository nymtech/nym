// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::{CancellationToken, DropGuard};
use tokio_util::task::TaskTracker;
use tracing::info;

pub const DEFAULT_MAX_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);

// pending name
//
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
    const OVERFLOW_NAME: &'static str = "reached maximum TaskClient children name depth";

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

    // expose the method with the old name for easier migration
    #[must_use]
    pub fn fork<S: Into<String>>(&self, child_suffix: S) -> Self {
        self.clone_with_suffix(child_suffix)
    }

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
    pub fn with_suffix<S: Into<String>>(self, suffix: S) -> Self {
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

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn catch_interrupt(&self) {
        crate::wait_for_signal().await;
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
}

pub struct ShutdownManager {
    pub root_token: ShutdownToken,
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
    pub fn new(root_token: impl Into<String>) -> Self {
        ShutdownManager {
            root_token: ShutdownToken::new(root_token),
            tracker: Default::default(),
            max_shutdown_duration: Default::default(),
        }
    }

    pub fn with_shutdown_duration(mut self, duration: Duration) -> Self {
        self.max_shutdown_duration = duration;
        self
    }

    pub async fn wait_for_shutdown(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        let interrupt_future = tokio::signal::ctrl_c();

        #[cfg(target_arch = "wasm32")]
        let interrupt_future = futures::future::pending::<()>();

        let wait_future = sleep(self.max_shutdown_duration);

        tokio::select! {
            _ = self.tracker.wait() => {
                info!("all registered tasks successfully shutdown")
            },
            _ = interrupt_future => {
                info!("forcing shutdown")
            },
            _ = wait_future => {
                info!("timeout reached, forcing shutdown");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn catch_interrupt(&self) {
        self.root_token.catch_interrupt().await;

        info!("sending cancellation");
        self.root_token.cancel();

        info!("waiting for tasks to finish... (press ctrl-c to force)");
        self.wait_for_shutdown().await;
    }
}
