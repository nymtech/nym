// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What happens to a message once it comes out the far end of the mixnet.
//!
//! The mirror of [`SimEndpoint`](crate::transport::SimEndpoint): a client is handed one of these by
//! its [`SimEnv`](crate::sim::env::SimEnv) and calls it, without knowing whether the run is
//! interested in what arrives. A live run says so and keeps nothing; a test keeps everything and
//! says nothing.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::client::ClientId;

/// Where a client's reassembled messages go.
pub trait SimLogging: Send {
    fn log(&self, client: ClientId, plaintext: &[u8]);
}

/// What the CLI runs: log it on stdout.
pub struct StdOutLogging;

impl SimLogging for StdOutLogging {
    fn log(&self, client: ClientId, plaintext: &[u8]) {
        tracing::info!(
            "[Client {client}] Received: {:?}",
            String::from_utf8_lossy(plaintext)
        );
    }
}

/// A message that reached its recipient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientTrace {
    /// Who received it.
    pub client: ClientId,
    /// What they got, after reassembly.
    pub plaintext: Vec<u8>,
}

/// What a test runs: every delivery to every client, in arrival order.
///
/// Cloning shares the record, so each client writes into the one the harness reads.
#[derive(Clone, Default)]
pub struct ClientTraces(Arc<Mutex<Vec<ClientTrace>>>);

impl ClientTraces {
    /// Recover rather than propagate: a poisoned lock means a client panicked, and that failure is
    /// the one worth surfacing.
    fn lock(&self) -> MutexGuard<'_, Vec<ClientTrace>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Every delivery so far, in arrival order.
    pub fn all(&self) -> Vec<ClientTrace> {
        self.lock().clone()
    }

    /// What `client` has received, in arrival order.
    pub fn for_client(&self, client: ClientId) -> Vec<Vec<u8>> {
        self.lock()
            .iter()
            .filter(|delivery| delivery.client == client)
            .map(|delivery| delivery.plaintext.clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}

impl SimLogging for ClientTraces {
    fn log(&self, client: ClientId, plaintext: &[u8]) {
        self.lock().push(ClientTrace {
            client,
            plaintext: plaintext.to_vec(),
        });
    }
}
