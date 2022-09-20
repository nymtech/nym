// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod shutdown;
pub mod signal;

pub use shutdown::{ShutdownListener, ShutdownNotifier};
pub use signal::wait_for_signal;
