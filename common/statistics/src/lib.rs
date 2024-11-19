// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Nym Statistics
//!
//! This crate contains basic statistics utilities and abstractions to be re-used and
//! applied throughout both the client and gateway implementations.
//! 
//! For now stats fall into one of two categories, statistics that are consumed locally (logged to
//! console, made available to GUI, etc.) and statistics meant to be aggregated remotely. The latter
//! is disabled by default. 

// In the future we could consider attempting something more fancy like a mpms keyword based pub_sub
// or something so that we can have more variety in the places that report stats, periods at which they
// get reported, and sinks that handle actually reporting the statistics events. It is kept simple for
// now to limit the places that require dependencies on things like TaskClient, InboundMessage, etc.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use std::future::Future;

use nym_crypto::asymmetric::ed25519;
use nym_task::TaskClient;
use sha2::Digest;
use nym_client_core_config_types::StatsReporting;
use tokio_util::sync::CancellationToken;

/// Client specific statistics interfaces and events.
pub mod clients;
/// Statistics related errors.
pub mod error;
/// Gateway specific statistics interfaces and events.
pub mod gateways;
/// Statistics reporting abstractions and implementations.
pub mod report;
/// Controller for coordinating stats threads
pub mod controller;

const CLIENT_ID_PREFIX: &str = "client_stats_id";

pub fn generate_client_stats_id(id_key: ed25519::PublicKey) -> String {
    generate_stats_id(CLIENT_ID_PREFIX, id_key.to_base58_string())
}

fn generate_stats_id<M: AsRef<[u8]>>(prefix: &str, id_seed: M) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(prefix);
    hasher.update(&id_seed);
    let output = hasher.finalize();
    format!("{:x}", output)
}

/// Collection object
/// 
/// Allows both the creation of a stats collection object as well as the use of an already existing downstream stats
/// object (e.g. when using the native client from the VPN client). 
pub enum StatsCollection {
    /// Allows an existing stats channel to be used for the downstream connections
    PreExisting(clients::ClientStatsSender),
    /// Indicates to create a new stats object with this configuration
    FromConfig(StatsReporting)
}

impl Default for StatsCollection {
    fn default() -> Self {
        Self::FromConfig(StatsReporting::default())
    }
}

pub enum Runtime {
	Token(CancellationToken),
	Task(TaskClient),
}

impl Runtime {
    /// Generic fn across Tokens and tasks indicating cancellation
    pub async fn cancelled(&mut self) {
        match self {
            Self::Token(token) => token.cancelled().await,
            Self::Task(task) => task.recv_with_delay().await,
        };
    }

    /// Pass through fn handling `task.recv_timeout``
    pub async fn recv_timeout(&mut self) {
        match self {
            Self::Token(_) => {},
            Self::Task(task) => task.recv_timeout().await,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future);
}
