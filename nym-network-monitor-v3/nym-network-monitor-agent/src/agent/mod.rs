// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::config::NodeTesterConfig;
use crate::agent::helpers::load_noise_key;
use nym_crypto::asymmetric::x25519;
use nym_pemstore::load_key;
use std::path::Path;
use std::sync::Arc;
use url::Url;
use zeroize::Zeroizing;

pub(crate) mod config;
pub(crate) mod helpers;
pub(crate) mod result;
pub(crate) mod tested_node;
pub(crate) mod tester;

/// A network monitor agent that receives test assignments from the orchestrator,
/// stress-tests individual nym-nodes, and reports results back.
pub(crate) struct NetworkMonitorAgent {
    /// Tester configuration controlling rates, timeouts, and addressing.
    tester_config: NodeTesterConfig,

    /// Address of the orchestrator for requesting work assignments
    orchestrator_address: Url,

    /// The tester's own Noise key pair, used to authenticate the egress connection.
    noise_key: Arc<x25519::KeyPair>,

    /// Bearer token required for requesting work assignments
    /// and submitting the results
    orchestrator_token: Zeroizing<String>,
}

impl NetworkMonitorAgent {
    /// Creates a new agent, loading the Noise key from disk and wrapping the
    /// orchestrator token in a zeroizing container.
    pub(crate) fn new<P: AsRef<Path>>(
        tester_config: NodeTesterConfig,
        noise_key_path: P,
        orchestrator_address: Url,
        orchestrator_token: String,
    ) -> anyhow::Result<Self> {
        Ok(NetworkMonitorAgent {
            tester_config,
            orchestrator_address,
            noise_key: load_noise_key(noise_key_path)?,
            orchestrator_token: Zeroizing::new(orchestrator_token),
        })
    }
}
