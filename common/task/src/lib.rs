// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod shutdown;
#[cfg(not(target_arch = "wasm32"))]
pub mod signal;
pub mod spawn;

// WIP(JON): those both need to be public?
pub use shutdown::{ShutdownListener, ShutdownNotifier, StatusReceiver, StatusSender};
#[cfg(not(target_arch = "wasm32"))]
pub use signal::{wait_for_signal, wait_for_signal_and_error};

pub use spawn::spawn_with_report_error;
