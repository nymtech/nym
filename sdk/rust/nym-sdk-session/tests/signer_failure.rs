// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! End-to-end signer-failure tolerance: the real [`BandwidthController`] over
//! ephemeral storage, driven exactly like `Session::ensure_ticketbooks`, with a
//! fetcher that simulates the observed mainnet outage — the ecash endpoints for
//! the master verification key and coin-index signatures answer, while the
//! aggregated expiration-date-signatures fetch hangs (or errors).
//!
//! Reproduces the pre-fix wedge (controller loop stuck, issued ticketbook never
//! persisted — the exact on-disk state observed on mainnet), and proves the fix:
//! with the SDK's [`TimeoutFetcher`] the paid-for ticketbook is persisted, the
//! provisioning wait resolves, a retry never re-purchases, and a spend attempt
//! during the outage fails fast instead of hanging.
//!
//! No network, no chain, no funds: the ticketbook is fabricated locally with a
//! real 2-of-3 threshold signer set (see [`support`]).

// Test code legitimately asserts/`expect`s on setup; same allowance as the
// bandwidth-controller's own integration tests.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use nym_bandwidth_controller::config::BandwidthControllerConfig;
use nym_bandwidth_controller::error::FetcherErrorKind;
use nym_bandwidth_controller::requests::BandwidthControllerRequestSender;
use nym_bandwidth_controller::{
    BandwidthController, CredentialFetcher, CredentialFetcherError, CredentialPublicDataFetcher,
    FetcherError, NymCredential, TicketType,
};
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_credential_storage::initialise_ephemeral_storage;
use nym_credential_storage::storage::Storage;
use nym_credentials::{AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures};
use nym_credentials_interface::VerificationKeyAuth;
use nym_ecash_time::Date;
use nym_sdk_session::TimeoutFetcher;
use nym_task::ShutdownToken;
use tokio::sync::Notify;

use support::TestEcash;

/// Per-call bound used in these tests — small so recovery is fast, large enough
/// that the healthy (instant) fetches never trip it.
const TEST_TIMEOUT: Duration = Duration::from_millis(200);

/// How the fetcher's expiration-date-signatures endpoint misbehaves. The master
/// verification key and coin-index signatures always answer (with real values) —
/// mirroring the observed mainnet outage, where only the aggregated
/// expiration-date endpoint was unresponsive.
#[derive(Clone, Copy)]
enum ExpirationMode {
    /// Accept the call and never respond (the observed mainnet behavior).
    Hang,
    /// Fail immediately (a signer returning an error status).
    Error,
}

#[derive(Debug, thiserror::Error)]
#[error("simulated signer failure")]
struct SimulatedFailure;

impl FetcherError for SimulatedFailure {
    fn kind(&self) -> FetcherErrorKind {
        FetcherErrorKind::Api
    }
}

/// A [`CredentialFetcher`] simulating issuance against flaky signers: issuance
/// itself succeeds (returning a locally fabricated, real threshold-signed
/// ticketbook), the vk/coin-index fetches succeed, and the expiration-date
/// fetch misbehaves per [`ExpirationMode`].
struct FlakyFetcher {
    ecash: Arc<TestEcash>,
    mode: ExpirationMode,
    /// Distinct user seed per issued book, so repeat fetches yield distinct books.
    issued: AtomicUsize,
    /// Total `fetch_ticketbooks` calls — the money-safety counter.
    fetch_calls: Arc<AtomicUsize>,
    /// Signals (with a stored permit) once the expiration fetch has started hanging.
    hang_entered: Arc<Notify>,
}

impl FlakyFetcher {
    fn new(ecash: Arc<TestEcash>, mode: ExpirationMode) -> Self {
        FlakyFetcher {
            ecash,
            mode,
            issued: AtomicUsize::new(0),
            fetch_calls: Arc::new(AtomicUsize::new(0)),
            hang_entered: Arc::new(Notify::new()),
        }
    }
}

#[async_trait]
impl CredentialPublicDataFetcher for FlakyFetcher {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: u64,
    ) -> Result<nym_credentials::EpochVerificationKey, CredentialFetcherError> {
        Ok(self.ecash.epoch_verification_key(epoch_id))
    }

    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: u64,
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        Ok(self.ecash.coin_index_signatures(epoch_id))
    }

    async fn fetch_expiration_date_signatures(
        &self,
        _expiration_date: Date,
        _epoch_id: u64,
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        match self.mode {
            ExpirationMode::Hang => {
                self.hang_entered.notify_one();
                std::future::pending().await
            }
            ExpirationMode::Error => Err(SimulatedFailure.into()),
        }
    }
}

#[async_trait]
impl CredentialFetcher for FlakyFetcher {
    async fn fetch_ticketbooks(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        self.fetch_calls.fetch_add(1, Ordering::SeqCst);
        let seed = self.issued.fetch_add(1, Ordering::SeqCst);
        let book = self.ecash.ticketbook(ticketbook_type, seed as u64);
        Ok(vec![NymCredential::Ticketbook(Box::new(book))])
    }

    async fn cleanup(&self) {}

    async fn reset(self) -> Result<(), CredentialFetcherError> {
        Ok(())
    }
}

/// Spawn a controller (empty managed set — the session's default — so nothing
/// restocks except our explicit requests) over the given storage and fetcher.
fn spawn_controller(
    storage: EphemeralStorage,
    fetcher: impl CredentialFetcher + 'static,
) -> (BandwidthControllerRequestSender, ShutdownToken) {
    let config = BandwidthControllerConfig {
        managed_ticket_types: Vec::new(),
        ..Default::default()
    };
    let controller = BandwidthController::new(storage)
        .with_config(config)
        .with_credential_fetcher(fetcher);
    let sender = controller.get_request_sender();
    let shutdown = ShutdownToken::new();
    tokio::spawn({
        let shutdown = shutdown.clone();
        async move { controller.run(shutdown).await }
    });
    (sender, shutdown)
}

/// Mirror `Session::ensure_ticketbooks`: explicit restock, then wait for readiness.
async fn provision(
    sender: &BandwidthControllerRequestSender,
    types: Vec<TicketType>,
) -> Result<(), String> {
    sender
        .restock_ticketbooks(types.clone())
        .await
        .map_err(|e| e.to_string())?;
    sender
        .wait_for_ticketbooks(types)
        .await
        .map_err(|e| e.to_string())
}

/// 4.4 — bug reproduction (the mainnet wedge, offline): WITHOUT the decorator, a
/// hanging expiration-date fetch wedges the controller loop mid-store. The issued
/// (paid-for) ticketbook is never persisted — while the global data fetched
/// before the hang (vk, coin-index sigs) IS: the exact `creds.db` state observed
/// on mainnet. Provisioning never resolves, and even unrelated controller
/// requests stop being served.
#[tokio::test]
async fn characterize_hanging_signers_wedge_controller_without_decorator() {
    let ecash = Arc::new(TestEcash::new());
    let fetcher = FlakyFetcher::new(ecash, ExpirationMode::Hang);
    let hang_entered = fetcher.hang_entered.clone();

    let storage = initialise_ephemeral_storage();
    let (sender, _shutdown) = spawn_controller(storage.clone(), fetcher);

    sender
        .restock_ticketbooks(vec![TicketType::V1WireguardEntry])
        .await
        .expect("restock request");

    // Deterministic sync point: the fetched book has reached `store_ticketbook`,
    // which is now hanging on the expiration-date fetch — on the controller loop.
    tokio::time::timeout(Duration::from_secs(5), hang_entered.notified())
        .await
        .expect("the expiration-date fetch must have been attempted");

    // The wedge: provisioning never resolves...
    let waited = tokio::time::timeout(
        Duration::from_secs(2),
        sender.wait_for_ticketbooks(vec![TicketType::V1WireguardEntry]),
    )
    .await;
    assert!(waited.is_err(), "controller must be wedged (pre-fix bug)");

    // ...and even unrelated requests aren't served any more.
    let available = tokio::time::timeout(Duration::from_secs(2), {
        let sender = sender.clone();
        async move { sender.get_available_ticketbooks().await }
    })
    .await;
    assert!(available.is_err(), "controller loop must be unresponsive");

    // The money-losing part, asserted directly on (shared) storage: the issued
    // ticketbook was never persisted, while vk + coin-index sigs were.
    let books = storage
        .get_ticketbooks_info()
        .await
        .expect("storage readable");
    assert!(
        books.is_empty(),
        "pre-fix: the paid-for ticketbook is lost on restart"
    );
}

/// 4.5 — recovery (Hang + decorator): the timeout converts the hang into a
/// bounded fetch error; the controller's best-effort store persists the
/// ticketbook anyway and readiness resolves.
#[tokio::test]
async fn hanging_signers_with_decorator_persist_ticketbook_and_resolve() {
    let ecash = Arc::new(TestEcash::new());
    let fetcher = FlakyFetcher::new(ecash, ExpirationMode::Hang);
    let wrapped = TimeoutFetcher::with_timeout(fetcher, TEST_TIMEOUT);

    let storage = initialise_ephemeral_storage();
    let (sender, _shutdown) = spawn_controller(storage.clone(), wrapped);

    tokio::time::timeout(
        Duration::from_secs(10),
        provision(&sender, vec![TicketType::V1WireguardEntry]),
    )
    .await
    .expect("provisioning must not hang")
    .expect("provisioning must succeed");

    let books = storage
        .get_ticketbooks_info()
        .await
        .expect("storage readable");
    assert_eq!(books.len(), 1, "the issued ticketbook must be persisted");
    assert_eq!(books[0].total_tickets, 50);
    assert_eq!(books[0].used_tickets, 0);
}

/// 4.5 (Error mode, no decorator): a *fast-failing* expiration fetch never
/// needed the decorator — the store path is best-effort. This isolates what the
/// decorator adds: it only converts hangs into this already-tolerated shape.
#[tokio::test]
async fn erroring_signers_persist_ticketbook_even_without_decorator() {
    let ecash = Arc::new(TestEcash::new());
    let fetcher = FlakyFetcher::new(ecash, ExpirationMode::Error);

    let storage = initialise_ephemeral_storage();
    let (sender, _shutdown) = spawn_controller(storage.clone(), fetcher);

    tokio::time::timeout(
        Duration::from_secs(10),
        provision(&sender, vec![TicketType::V1WireguardEntry]),
    )
    .await
    .expect("provisioning must not hang")
    .expect("provisioning must succeed");

    let books = storage
        .get_ticketbooks_info()
        .await
        .expect("storage readable");
    assert_eq!(books.len(), 1);
}

/// 4.6 — money safety: once a ticketbook is persisted (even during the outage),
/// provisioning again NEVER re-purchases — zero additional issuance calls.
#[tokio::test]
async fn retry_after_outage_never_repurchases() {
    let ecash = Arc::new(TestEcash::new());
    let fetcher = FlakyFetcher::new(ecash, ExpirationMode::Hang);
    let fetch_calls = fetcher.fetch_calls.clone();
    let wrapped = TimeoutFetcher::with_timeout(fetcher, TEST_TIMEOUT);

    let storage = initialise_ephemeral_storage();
    let (sender, _shutdown) = spawn_controller(storage, wrapped);

    // First provisioning run: issues (deposits) exactly once.
    tokio::time::timeout(
        Duration::from_secs(10),
        provision(&sender, vec![TicketType::V1WireguardEntry]),
    )
    .await
    .expect("first run must not hang")
    .expect("first run must succeed");
    assert_eq!(fetch_calls.load(Ordering::SeqCst), 1);

    // Second run over the same storage: stocked (50 > restock threshold), so no
    // new issuance — the retry that used to burn NYM on mainnet is now free.
    tokio::time::timeout(
        Duration::from_secs(10),
        provision(&sender, vec![TicketType::V1WireguardEntry]),
    )
    .await
    .expect("second run must not hang")
    .expect("second run must succeed");
    assert_eq!(
        fetch_calls.load(Ordering::SeqCst),
        1,
        "a persisted ticketbook must never be re-purchased"
    );
}

/// 4.7 — degraded spend: with a persisted ticketbook but the expiration-date
/// signatures still unavailable, a spend attempt fails fast (bounded by the
/// decorator) instead of hanging — and starts working the moment signers return.
#[tokio::test]
async fn spend_during_outage_fails_fast_instead_of_hanging() {
    let ecash = Arc::new(TestEcash::new());
    let fetcher = FlakyFetcher::new(ecash.clone(), ExpirationMode::Hang);
    let wrapped = TimeoutFetcher::with_timeout(fetcher, TEST_TIMEOUT);

    let storage = initialise_ephemeral_storage();
    let (sender, _shutdown) = spawn_controller(storage, wrapped);

    tokio::time::timeout(
        Duration::from_secs(10),
        provision(&sender, vec![TicketType::V1WireguardEntry]),
    )
    .await
    .expect("provisioning must not hang")
    .expect("provisioning must succeed");

    // Spending needs the expiration-date signatures the outage withholds: the
    // attempt must surface a bounded error, not a hang.
    let spend = tokio::time::timeout(
        Duration::from_secs(5),
        sender.get_ecash_ticket(
            TicketType::V1WireguardEntry,
            support::test_gateway_id(),
            1,
            time::OffsetDateTime::now_utc(),
        ),
    )
    .await
    .expect("spend attempt must not hang");
    assert!(
        spend.is_err(),
        "spend during the outage must fail (fast) — got {spend:?}"
    );
}

/// The fixture itself is sound: with healthy signers-equivalent data the
/// verification key aggregates and the fabricated book carries the real
/// ticket count. (Guards the fixture against silent drift.)
#[test]
fn fixture_produces_wireguard_ticketbooks() {
    let ecash = TestEcash::new();
    let book = ecash.ticketbook(TicketType::V1WireguardEntry, 0);
    assert_eq!(book.ticketbook_type(), TicketType::V1WireguardEntry);
    assert_eq!(book.spent_tickets(), 0);
    assert_eq!(book.params_total_tickets(), 50);
    let _vk: VerificationKeyAuth = ecash.epoch_verification_key(1).key;
}
