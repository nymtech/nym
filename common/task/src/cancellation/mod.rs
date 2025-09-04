// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A [CancellationToken](tokio_util::sync::CancellationToken)-backed shutdown mechanism for Nym binaries.
//!
//! It allows creation of a centralised manager for keeping track of all signals that are meant
//! to trigger exit of all associated tasks and sending cancellation to the aforementioned futures.
//!
//! # Default usage
//!
//! ```no_run
//!     use std::time::Duration;
//!     use tokio::time::sleep;
//!     use nym_task::{ShutdownManager, ShutdownToken};
//!
//!     async fn my_task() {
//!         loop {
//!             sleep(Duration::from_secs(5)).await
//!             // do some periodic work that can be easily interrupted
//!         }
//!     }
//!
//!     async fn important_work_that_cant_be_interrupted() {}
//!
//!     async fn my_managed_task(shutdown_token: ShutdownToken) {
//!         tokio::select! {
//!             _ = shutdown_token.cancelled() => {}
//!             _ = important_work_that_cant_be_interrupted() => {}
//!         }
//!     }
//! #[tokio::main]
//! async fn main() {
//!     let shutdown_manager = ShutdownManager::build_new_default().expect("failed to register default shutdown signals");
//!
//!     let shutdown_token = shutdown_manager.child_shutdown_token();
//!     shutdown_manager.try_spawn_named(async move { my_managed_task(shutdown_token).await }, "important-managed-task");
//!     shutdown_manager.try_spawn_named_with_shutdown(my_task(), "another-task");
//!
//!     // wait for shutdown signal
//!     shutdown_manager.run_until_shutdown().await;
//! }
//! ```

use std::time::Duration;

pub mod manager;
pub mod token;
pub mod tracker;

pub use manager::ShutdownManager;
pub use token::{ShutdownDropGuard, ShutdownToken};
pub use tracker::ShutdownTracker;

pub const DEFAULT_MAX_SHUTDOWN_DURATION: Duration = Duration::from_secs(5);
