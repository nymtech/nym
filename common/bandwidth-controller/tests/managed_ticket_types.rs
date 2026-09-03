// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Proves the controller only proactively restocks (and therefore only deposits for) the ticket
//! types in `BandwidthControllerConfig::managed_ticket_types`. A wireguard-scoped controller must
//! never deposit for mixnet types, and an empty managed set must never deposit at all.

// Test code legitimately asserts/`expect`s on setup; the workspace denies `expect_used`
// (see `mock.rs` for the same allowance). The mutex locks below are poisoned-lock-safe (no unwrap).
#![allow(clippy::expect_used)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use nym_bandwidth_controller::config::BandwidthControllerConfig;
use nym_bandwidth_controller::error::FetcherErrorKind;
use nym_bandwidth_controller::{
    BandwidthController, CredentialFetcher, CredentialFetcherError, CredentialPublicDataFetcher,
    FetcherError, NymCredential, TicketType,
};
use nym_credential_storage::initialise_ephemeral_storage;
use nym_task::ShutdownToken;

/// Records every ticket type it is asked to fetch, and returns no credentials (so the controller's
/// stock never actually rises and the recording reflects exactly what the restock logic requested).
#[derive(Default)]
struct RecordingFetcher {
    requested: Arc<Mutex<Vec<TicketType>>>,
}

impl RecordingFetcher {
    fn record(&self, ticketbook_type: TicketType) {
        // Recover a poisoned lock rather than `unwrap()` (CI denies `clippy::unwrap-used`).
        self.requested
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ticketbook_type);
    }
}

/// Snapshot the recorded requests without `unwrap()` on a possibly-poisoned lock.
fn recorded(requested: &Arc<Mutex<Vec<TicketType>>>) -> Vec<TicketType> {
    requested
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[derive(Debug, thiserror::Error)]
#[error("unused test fetcher error")]
struct TestFetcherError;

impl FetcherError for TestFetcherError {
    fn kind(&self) -> FetcherErrorKind {
        FetcherErrorKind::Other
    }
}

#[async_trait]
impl CredentialFetcher for RecordingFetcher {
    async fn fetch_ticketbooks(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        self.record(ticketbook_type);
        Ok(Vec::new())
    }

    async fn cleanup(&self) {}

    async fn reset(self) -> Result<(), CredentialFetcherError> {
        Ok(())
    }
}

#[async_trait]
impl CredentialPublicDataFetcher for RecordingFetcher {
    async fn fetch_master_verification_key(
        &self,
        _epoch_id: u64,
    ) -> Result<nym_credentials::EpochVerificationKey, CredentialFetcherError> {
        Err(Box::new(TestFetcherError))
    }

    async fn fetch_coin_index_signatures(
        &self,
        _epoch_id: u64,
    ) -> Result<nym_credentials::AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        Err(Box::new(TestFetcherError))
    }

    async fn fetch_expiration_date_signatures(
        &self,
        _expiration_date: nym_ecash_time::Date,
        _epoch_id: u64,
    ) -> Result<nym_credentials::AggregatedExpirationDateSignatures, CredentialFetcherError> {
        Err(Box::new(TestFetcherError))
    }
}

fn config_with_managed(managed: Vec<TicketType>) -> BandwidthControllerConfig {
    BandwidthControllerConfig {
        managed_ticket_types: managed,
        ..Default::default()
    }
}

/// Installing a fetcher into a controller scoped (via config) to the wireguard ticket types must
/// trigger restock requests only for those types — never for mixnet types.
#[tokio::test]
async fn scoped_controller_restocks_only_managed_types() {
    let storage = initialise_ephemeral_storage();
    let requested = Arc::new(Mutex::new(Vec::new()));
    let fetcher = RecordingFetcher {
        requested: requested.clone(),
    };

    let managed = vec![TicketType::V1WireguardEntry, TicketType::V1WireguardExit];
    let controller =
        BandwidthController::new(storage).with_config(config_with_managed(managed.clone()));
    let sender = controller.get_request_sender();

    let shutdown = ShutdownToken::new();
    let run_handle = tokio::spawn({
        let shutdown = shutdown.clone();
        async move { controller.run(shutdown).await }
    });

    // installing the fetcher immediately restocks any low managed type (all are low on empty storage)
    sender
        .set_credential_fetcher(Arc::new(fetcher))
        .await
        .expect("set fetcher");

    // wait for the spawned restock fetches to run and be recorded
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let seen: HashSet<_> = recorded(&requested).into_iter().collect();
        if seen.len() >= managed.len() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for restock fetches"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let seen: HashSet<_> = recorded(&requested).into_iter().collect();
    assert_eq!(
        seen,
        managed.into_iter().collect::<HashSet<_>>(),
        "restock requested exactly the managed types"
    );
    // the hard guarantee: no mixnet ticket type was ever requested
    assert!(!seen.contains(&TicketType::V1MixnetEntry));
    assert!(!seen.contains(&TicketType::V1MixnetExit));

    shutdown.cancel();
    let _ = run_handle.await;
}

/// With an empty managed set, installing a fetcher must NOT trigger any background deposit — the
/// controller only spends existing stock. This backs the session's opt-out (no auto-restock) mode.
#[tokio::test]
async fn empty_managed_set_makes_no_deposits() {
    let storage = initialise_ephemeral_storage();
    let requested = Arc::new(Mutex::new(Vec::new()));
    let fetcher = RecordingFetcher {
        requested: requested.clone(),
    };

    let controller = BandwidthController::new(storage).with_config(config_with_managed(vec![]));
    let sender = controller.get_request_sender();

    let shutdown = ShutdownToken::new();
    let run_handle = tokio::spawn({
        let shutdown = shutdown.clone();
        async move { controller.run(shutdown).await }
    });

    sender
        .set_credential_fetcher(Arc::new(fetcher))
        .await
        .expect("set fetcher");

    // give any (erroneous) restock a chance to fire, then assert nothing was requested
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        recorded(&requested).is_empty(),
        "no deposits must be made when the managed set is empty"
    );

    shutdown.cancel();
    let _ = run_handle.await;
}
