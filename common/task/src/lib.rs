// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod cancellation;
pub mod connections;
pub mod event;
pub mod manager;
pub(crate) mod runtime_registry;
#[cfg(not(target_arch = "wasm32"))]
pub mod signal;
pub mod spawn;

pub use cancellation::{ShutdownDropGuard, ShutdownManager, ShutdownToken, ShutdownTracker};
pub use event::{StatusReceiver, StatusSender, TaskStatus, TaskStatusEvent};
#[allow(deprecated)]
pub use manager::{TaskClient, TaskManager};
pub use spawn::spawn_future;
pub use tokio_util::task::TaskTracker;

#[cfg(not(target_arch = "wasm32"))]
pub use signal::{wait_for_signal, wait_for_signal_and_error};

/// Get or create a ShutdownTracker for SDK use.
/// This provides automatic task management without requiring manual setup.
pub async fn get_sdk_shutdown_tracker() -> ShutdownTracker {
    runtime_registry::RuntimeRegistry::get_or_create_sdk()
        .await
        .shutdown_tracker_owned()
}
