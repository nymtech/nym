// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod cancellation;
pub mod connections;
pub mod event;
pub mod manager;
#[cfg(not(target_arch = "wasm32"))]
pub mod signal;
pub mod spawn;

pub use cancellation::{ShutdownDropGuard, ShutdownManager, ShutdownToken};
pub use event::{StatusReceiver, StatusSender, TaskStatus, TaskStatusEvent};
pub use manager::{TaskClient, TaskHandle, TaskManager};
pub use spawn::{spawn, spawn_with_report_error};
pub use tokio_util::task::TaskTracker;

#[cfg(not(target_arch = "wasm32"))]
pub use signal::{wait_for_signal, wait_for_signal_and_error};
