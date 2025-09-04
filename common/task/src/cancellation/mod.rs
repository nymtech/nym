// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;
use std::future::Future;
use std::time::Duration;
pub mod manager;
pub mod token;
pub mod tracker;

pub use manager::ShutdownManager;
pub use token::{ShutdownDropGuard, ShutdownToken};
pub use tracker::ShutdownTracker;

pub const DEFAULT_MAX_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);
