// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod connections;
pub mod manager;
#[cfg(not(target_arch = "wasm32"))]
pub mod signal;
pub mod spawn;

pub use manager::{StatusReceiver, StatusSender, TaskClient, TaskManager};
#[cfg(not(target_arch = "wasm32"))]
pub use signal::wait_for_signal_and_error;

pub use spawn::spawn_with_report_error;
