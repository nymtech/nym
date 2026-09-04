//! Session-level reuse tests (task 4.2 of `dvpn-registration-reuse`).
//!
//! These exercise the cache-consultation seam offline: a `Session` built
//! with an external (counting) bandwidth provider needs no chain, no
//! controller, and no gateway. The full `register_*` paths additionally
//! need a live nym-api (topology) + gateway (LP exchange) and are covered
//! by the documented manual mainnet validation instead — the decision
//! logic they share (`cached_hop` / `persist_registration` /
//! `invalidate_registration`) is what is tested here.

use super::*;
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_bandwidth_controller::{PreparedCredential, PreparedCredentialMetadata};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::registration_cache::RegistrationCache;

/// A provider that only counts spends — any spend during a cache-served
/// registration is a test failure.
#[derive(Default)]
struct CountingProvider {
    spends: AtomicUsize,
}

#[async_trait::async_trait]
impl BandwidthTicketProvider for CountingProvider {
    async fn get_ecash_ticket(
        &self,
        _ticket_type: TicketType,
        _gateway_id: ed25519::PublicKey,
        _tickets_to_spend: u32,
        _spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        self.spends.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        Ok(None)
    }

    async fn attempt_revert_spending(
        &self,
        _metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        Ok(false)
    }

    async fn close(&self) {}
}

fn test_mnemonic() -> bip39::Mnemonic {
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        .parse()
        .unwrap()
}

fn gateway_identity(seed: u8) -> ed25519::PublicKey {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::from_seed([seed; 32]);
    *ed25519::KeyPair::new(&mut rng).public_key()
}

fn gateway_info(identity: ed25519::PublicKey) -> GatewayInfo {
    GatewayInfo {
        identity,
        node_id: 1,
        country: None,
        ip: std::net::IpAddr::from([192, 0, 2, 1]),
        name: None,
    }
}

fn wg_config() -> WireguardConfiguration {
    WireguardConfiguration {
        public_key: x25519::PrivateKey::from_secret([9; 32]).public_key(),
        psk: None,
        endpoint: std::net::SocketAddr::from(([192, 0, 2, 1], 51822)),
        private_ipv4: std::net::Ipv4Addr::new(10, 1, 2, 3),
        private_ipv6: std::net::Ipv6Addr::LOCALHOST,
    }
}

fn tempdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nym-session-reuse-test-{}-{:x}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Offline session: the external provider skips chain/controller/store
/// construction entirely, and mainnet network details are static.
async fn offline_session(
    data_path: PathBuf,
    reuse: bool,
    provider: Arc<CountingProvider>,
) -> Session {
    offline_session_with_topology(data_path, reuse, provider, true).await
}

/// [`offline_session`] with an explicit `SessionConfig::two_hop`.
async fn offline_session_with_topology(
    data_path: PathBuf,
    reuse: bool,
    provider: Arc<CountingProvider>,
    two_hop: bool,
) -> Session {
    Session::new(
        SessionConfig {
            mnemonic: test_mnemonic(),
            network: NymNetworkDetails::new_mainnet(),
            credential_store_path: None,
            data_path,
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: Some(provider),
            reuse_registrations: reuse,
            two_hop,
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
}

/// A session configured single-hop rejects every two-hop registration entry point up front, with
/// the dedicated topology error and before any network work (this test is fully offline).
#[tokio::test]
async fn single_hop_session_rejects_two_hop_registration() {
    let dir = tempdir();
    let provider = Arc::new(CountingProvider::default());
    let session = offline_session_with_topology(dir, true, provider.clone(), false).await;
    let avoid = [gateway_identity(9)];

    let attempts = [
        session
            .register_two_hop(&GatewaySpec::Random, &GatewaySpec::Random)
            .await,
        session
            .register_two_hop_avoiding_entries(&GatewaySpec::Random, &GatewaySpec::Random, &avoid)
            .await,
        session
            .register_two_hop_quic(&GatewaySpec::Random, &GatewaySpec::Random)
            .await,
        session
            .register_two_hop_quic_avoiding_entries(
                &GatewaySpec::Random,
                &GatewaySpec::Random,
                &avoid,
            )
            .await,
    ];
    for res in attempts {
        assert!(
            matches!(res, Err(SessionError::TopologyMismatch)),
            "single-hop session must reject two-hop registration with TopologyMismatch"
        );
    }
    assert_eq!(
        provider.spends.load(Ordering::SeqCst),
        0,
        "a rejected registration spends nothing"
    );
}

/// A seeded cache entry is served by `cached_hop` — assembled into a
/// `HopConfig` with the persisted key/config — with ZERO provider spends.
#[tokio::test]
async fn cached_hop_is_served_without_spending() {
    let dir = tempdir();
    let gw = gateway_identity(1);
    let key = x25519::PrivateKey::from_secret([7; 32]);
    RegistrationCache::load(&dir, "mainnet").insert(&gw, WgRole::Entry, &key, &wg_config());

    let provider = Arc::new(CountingProvider::default());
    let session = offline_session(dir, true, provider.clone()).await;

    let hop = session
        .cached_hop(&gw, gateway_info(gw), WgRole::Entry)
        .expect("cache hit");
    assert_eq!(hop.client_private_key.to_bytes(), [7; 32]);
    assert_eq!(hop.wg_config, wg_config());
    assert_eq!(hop.gateway_identity, gw);
    assert_eq!(
        provider.spends.load(Ordering::SeqCst),
        0,
        "a cache-served hop must never spend"
    );
}

/// With reuse disabled, the cache is neither read (seeded entries are
/// ignored) nor written (`persist_registration` is a no-op).
#[tokio::test]
async fn opt_out_neither_reads_nor_writes() {
    let dir = tempdir();
    let gw = gateway_identity(1);
    let key = x25519::PrivateKey::from_secret([7; 32]);
    RegistrationCache::load(&dir, "mainnet").insert(&gw, WgRole::Entry, &key, &wg_config());

    let provider = Arc::new(CountingProvider::default());
    let session = offline_session(dir.clone(), false, provider).await;

    // seeded entry is ignored
    assert!(session
        .cached_hop(&gw, gateway_info(gw), WgRole::Entry)
        .is_none());

    // and nothing is persisted (`finalize_hop` assembles but skips the cache)
    let gw2 = gateway_identity(2);
    let _ = session.finalize_hop(
        &gw2,
        gateway_info(gw2),
        WgRole::Exit,
        x25519::PrivateKey::from_secret([7; 32]),
        wg_config(),
    );
    assert!(RegistrationCache::load(&dir, "mainnet")
        .lookup(&gw2, WgRole::Exit)
        .is_none());
}

/// `invalidate_registration` removes exactly the keyed entry, persistently
/// — the fallback path after a cached peer fails to establish.
#[tokio::test]
async fn invalidation_enables_fresh_registration() {
    let dir = tempdir();
    let gw = gateway_identity(1);
    let other = gateway_identity(2);
    let key = x25519::PrivateKey::from_secret([7; 32]);
    {
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw, WgRole::Entry, &key, &wg_config());
        cache.insert(&other, WgRole::Exit, &key, &wg_config());
    }

    let provider = Arc::new(CountingProvider::default());
    let session = offline_session(dir.clone(), true, provider).await;
    session.invalidate_registration(&gw, WgRole::Entry);

    assert!(session
        .cached_hop(&gw, gateway_info(gw), WgRole::Entry)
        .is_none());
    assert!(session
        .cached_hop(&other, gateway_info(other), WgRole::Exit)
        .is_some());
    // persisted: a new session over the same dir agrees
    let session2 = offline_session(dir, true, Arc::new(CountingProvider::default())).await;
    assert!(session2
        .cached_hop(&gw, gateway_info(gw), WgRole::Entry)
        .is_none());
}

/// `finalize_hop` + `cached_hop` round-trip through the session's own APIs
/// (what the register paths do on a fresh registration).
#[tokio::test]
async fn persist_then_reuse_round_trip() {
    let dir = tempdir();
    let gw = gateway_identity(1);

    let provider = Arc::new(CountingProvider::default());
    let session = offline_session(dir.clone(), true, provider).await;
    let _ = session.finalize_hop(
        &gw,
        gateway_info(gw),
        WgRole::Exit,
        x25519::PrivateKey::from_secret([7; 32]),
        wg_config(),
    );

    // same session sees it...
    assert!(session
        .cached_hop(&gw, gateway_info(gw), WgRole::Exit)
        .is_some());
    // ...and so does a later one over the same data dir (restart survival)
    let session2 = offline_session(dir, true, Arc::new(CountingProvider::default())).await;
    let hop = session2
        .cached_hop(&gw, gateway_info(gw), WgRole::Exit)
        .expect("survives restart");
    assert_eq!(hop.client_private_key.to_bytes(), [7; 32]);
}

// ---------------------------------------------------------------------------
// Fetcher-lifecycle provisioning (change `fix-dvpn-session-fetcher-restock`).
//
// These drive `OwnedController::ensure` against a real, running `BandwidthController` (ephemeral
// store, managed = WireGuard types) with a recording mock fetcher, so the install → fetch →
// removal lifecycle is exercised deterministically without minting real ecash. The full "provisions
// successfully and passes traffic" path needs live signers + a gateway and is covered by the gated
// `smoldvpn/tests/live_bringup.rs` integration tests.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Provisioning tests: the two controller modes, offline (ephemeral store, locally threshold-signed
// ticketbooks, a recording fetcher — no chain, no network, no funds).
//
// `BandwidthController`, `BandwidthControllerConfig`, `BandwidthControllerError`,
// `ControllerMode`, `CredentialFetcher`, `SessionError`, `ShutdownToken`, `Storage`, `TicketType`,
// `Arc`, `await_stocked`, `needed_ticket_types`, `provision_once` and `read_stock` come in via
// `use super::*`.
// ---------------------------------------------------------------------------------------------

use nym_bandwidth_controller::error::FetcherErrorKind;
use nym_bandwidth_controller::{
    CredentialFetcherError, CredentialPublicDataFetcher, FetcherError, NymCredential,
};
use nym_credential_storage::initialise_ephemeral_storage;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_ecash_time::Date;
use nym_validator_client::nym_api::EpochId;
use std::time::Duration;

/// Local threshold-signer fixtures shared with the integration tests: real ticketbooks and the
/// global signing data a healthy signer set would serve.
#[path = "../tests/support/mod.rs"]
mod support;
use support::TestEcash;

/// What the recording fetcher does on a ticketbook fetch.
#[derive(Clone, Copy)]
enum FetchMode {
    /// Issue a real (locally signed) ticketbook of the requested type.
    Issue,
    /// Fail the first fetch, issue on every later one.
    FailOnceThenIssue,
    /// Return an empty batch (nothing gets stored).
    Empty,
    /// Fail every fetch.
    Fail,
}

/// Call log shared by a test and its fetcher.
#[derive(Default)]
struct FetcherCalls {
    /// The ticket types fetched, in order.
    fetches: std::sync::Mutex<Vec<TicketType>>,
    cleanups: AtomicUsize,
}

impl FetcherCalls {
    fn fetches(&self) -> Vec<TicketType> {
        self.fetches.lock().unwrap().clone()
    }
    fn cleanups(&self) -> usize {
        self.cleanups.load(Ordering::SeqCst)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("recorded fetch failure")]
struct RecordedFailure;

impl FetcherError for RecordedFailure {
    fn kind(&self) -> FetcherErrorKind {
        FetcherErrorKind::Api
    }
}

/// A fetcher that records its calls and serves the local fixtures instead of talking to a chain.
/// It models the real `NyxdCredentialFetcher`'s `cleanup` (which closes the recovery store): once
/// cleaned up, any further fetch fails, so a test that used a torn-down fetcher would fail loudly.
#[derive(Clone)]
struct RecordingFetcher {
    calls: Arc<FetcherCalls>,
    ecash: Arc<TestEcash>,
    mode: FetchMode,
    failed_once: Arc<AtomicBool>,
    cleaned: Arc<AtomicBool>,
}

impl RecordingFetcher {
    fn new(calls: Arc<FetcherCalls>, ecash: Arc<TestEcash>, mode: FetchMode) -> Self {
        Self {
            calls,
            ecash,
            mode,
            failed_once: Arc::new(AtomicBool::new(false)),
            cleaned: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait::async_trait]
impl CredentialFetcher for RecordingFetcher {
    async fn fetch_ticketbooks(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        if self.cleaned.load(Ordering::SeqCst) {
            return Err(RecordedFailure.into());
        }
        let seed = {
            let mut fetches = self.calls.fetches.lock().unwrap();
            fetches.push(ticketbook_type);
            fetches.len() as u64
        };
        match self.mode {
            FetchMode::Fail => Err(RecordedFailure.into()),
            FetchMode::Empty => Ok(Vec::new()),
            FetchMode::FailOnceThenIssue if !self.failed_once.swap(true, Ordering::SeqCst) => {
                Err(RecordedFailure.into())
            }
            FetchMode::Issue | FetchMode::FailOnceThenIssue => Ok(vec![NymCredential::Ticketbook(
                Box::new(self.ecash.ticketbook(ticketbook_type, seed)),
            )]),
        }
    }

    async fn cleanup(&self) {
        self.cleaned.store(true, Ordering::SeqCst);
        self.calls.cleanups.fetch_add(1, Ordering::SeqCst);
    }

    async fn reset(self) -> Result<(), CredentialFetcherError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl CredentialPublicDataFetcher for RecordingFetcher {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, CredentialFetcherError> {
        Ok(self.ecash.epoch_verification_key(epoch_id))
    }
    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        Ok(self.ecash.coin_index_signatures(epoch_id))
    }
    async fn fetch_expiration_date_signatures(
        &self,
        _expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        // the fixtures live on the default expiration date, which is also what every fixture
        // ticketbook carries
        let (expiration_date, signatures) = self.ecash.expiration_date_signatures();
        Ok(AggregatedExpirationDateSignatures {
            epoch_id,
            expiration_date,
            signatures,
        })
    }
}

/// The controller config a session builds for `two_hop`: default thresholds, topology-scoped
/// managed types.
fn controller_config(two_hop: bool) -> BandwidthControllerConfig {
    BandwidthControllerConfig {
        managed_ticket_types: needed_ticket_types(two_hop),
        ..Default::default()
    }
}

/// One-shot: a required type whose stock is above the restock threshold is never fetched; only the
/// missing type is issued, and the short-lived controller is torn down (fetcher cleaned up once).
#[tokio::test]
async fn one_shot_skips_stocked_type() {
    let ecash = Arc::new(TestEcash::new());
    let storage = initialise_ephemeral_storage();
    storage
        .insert_issued_ticketbook(&ecash.ticketbook(TicketType::V1WireguardEntry, 1_000))
        .await
        .unwrap();
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::Issue);

    provision_once(
        storage.clone(),
        fetcher,
        controller_config(true),
        needed_ticket_types(true),
    )
    .await
    .expect("provisioning succeeds");

    assert_eq!(
        calls.fetches(),
        vec![TicketType::V1WireguardExit],
        "only the exhausted exit type is fetched; the stocked entry type is not"
    );
    assert_eq!(
        calls.cleanups(),
        1,
        "teardown cleans the fetcher up exactly once"
    );
}

/// One-shot: with nothing stored, each required type is issued exactly once, both are usable
/// afterwards, and teardown runs.
#[tokio::test]
async fn one_shot_issues_each_low_type_once_and_tears_down() {
    let ecash = Arc::new(TestEcash::new());
    let storage = initialise_ephemeral_storage();
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::Issue);
    let config = controller_config(true);

    provision_once(
        storage.clone(),
        fetcher,
        config.clone(),
        needed_ticket_types(true),
    )
    .await
    .expect("provisioning succeeds");

    assert_eq!(
        calls.fetches(),
        vec![TicketType::V1WireguardEntry, TicketType::V1WireguardExit],
        "one fetch per required type, in order"
    );
    assert_eq!(
        calls.cleanups(),
        1,
        "teardown cleans the fetcher up exactly once"
    );
    let stock = read_stock(&storage).await.unwrap();
    for typ in needed_ticket_types(true) {
        assert!(
            stock.contains_minimal_tickets(typ, &config),
            "{typ} must be usable after provisioning"
        );
    }
}

/// One-shot: fully stocked ⇒ no fetch at all (no deposit), still a clean teardown.
#[tokio::test]
async fn one_shot_is_a_noop_when_stocked() {
    let ecash = Arc::new(TestEcash::new());
    let storage = initialise_ephemeral_storage();
    for (i, typ) in needed_ticket_types(true).into_iter().enumerate() {
        storage
            .insert_issued_ticketbook(&ecash.ticketbook(typ, 2_000 + i as u64))
            .await
            .unwrap();
    }
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::Issue);

    provision_once(
        storage,
        fetcher,
        controller_config(true),
        needed_ticket_types(true),
    )
    .await
    .expect("nothing to do is a success");

    assert!(
        calls.fetches().is_empty(),
        "a stocked session must not deposit"
    );
    assert_eq!(calls.cleanups(), 1);
}

/// One-shot: a failing issuance surfaces as an error, stops at the first failure, and the
/// controller is still torn down.
#[tokio::test]
async fn one_shot_failure_still_tears_down() {
    let ecash = Arc::new(TestEcash::new());
    let storage = initialise_ephemeral_storage();
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::Fail);

    let res = provision_once(
        storage,
        fetcher,
        controller_config(true),
        needed_ticket_types(true),
    )
    .await;

    assert!(
        matches!(res, Err(SessionError::Issuance(_))),
        "a failing fetch must surface as an issuance error, got {res:?}"
    );
    assert_eq!(
        calls.fetches().len(),
        1,
        "issuance stops at the first failed type"
    );
    assert_eq!(calls.cleanups(), 1, "teardown runs on the failure path too");
}

/// One-shot: a fetch that "succeeds" without leaving a usable ticketbook is an error — the
/// requested types are verified usable after issuance.
#[tokio::test]
async fn one_shot_unusable_after_issuance_is_an_error() {
    let ecash = Arc::new(TestEcash::new());
    let storage = initialise_ephemeral_storage();
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::Empty);

    let res = provision_once(
        storage,
        fetcher,
        controller_config(false),
        needed_ticket_types(false),
    )
    .await;

    match res {
        Err(SessionError::Issuance(msg)) => assert!(
            msg.contains("no usable"),
            "error must name the unusable type: {msg}"
        ),
        other => panic!("expected an issuance error, got {other:?}"),
    }
    assert_eq!(calls.cleanups(), 1);
}

/// One-shot: a caller queued behind another provision (the session lock is held) is still
/// cancellable — it returns `Cancelled` promptly instead of blocking for the holder's whole budget,
/// and never starts a provision of its own. Building a one-shot session is offline (no network at
/// construction), so this runs without chain access.
#[tokio::test]
async fn ensure_lock_wait_is_cancellable() {
    let cancel = CancellationToken::new();
    let session = Session::new(
        SessionConfig {
            mnemonic: test_mnemonic(),
            network: NymNetworkDetails::new_mainnet(),
            credential_store_path: None,
            data_path: tempdir(),
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: None,
            reuse_registrations: false,
            two_hop: false,
        },
        cancel.clone(),
    )
    .await
    .expect("one-shot session builds offline");

    let Some(ControllerMode::OneShot { lock, .. }) = &session.mode else {
        panic!("default session must be in one-shot mode");
    };
    let held = lock.lock().await;
    cancel.cancel();

    let res = tokio::time::timeout(Duration::from_secs(5), session.ensure_ticketbooks(false))
        .await
        .expect("a cancelled caller must not block behind the held provisioning lock");
    assert!(
        matches!(res, Err(SessionError::Cancelled)),
        "queued caller must surface Cancelled, got {res:?}"
    );
    drop(held);
    session.shutdown().await;
}

/// Running mode: the controller is built with the fetcher installed and its startup sweep issues.
/// When that startup fetch has failed — so the required type is neither stocked nor in flight —
/// the readiness wait falls back to exactly one explicit restock and waits again.
#[tokio::test]
async fn running_mode_recovers_from_failed_startup_fetch() {
    let ecash = Arc::new(TestEcash::new());
    let calls = Arc::new(FetcherCalls::default());
    let fetcher = RecordingFetcher::new(calls.clone(), ecash, FetchMode::FailOnceThenIssue);
    let controller = BandwidthController::new(initialise_ephemeral_storage())
        .with_config(controller_config(false))
        .with_credential_fetcher(fetcher);
    let sender = controller.get_request_sender();
    let cancel = CancellationToken::new();
    let task = tokio::spawn(controller.run(ShutdownToken::new_from_tokio_token(cancel.clone())));

    // Let the startup sweep's fetch run, fail and settle. Gate on the fetch having happened first:
    // the sweep's first tick is not guaranteed to be processed before an already-queued readiness
    // request, so an immediate wait could see `Unavailable` before any fetch was attempted. A waiter
    // parked while the fetch is in flight gets the failure itself; once it has been drained, the type
    // is neither stocked nor in flight.
    let types = needed_ticket_types(false);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if !calls.fetches().is_empty() {
                match sender.wait_for_ticketbooks(types.clone()).await {
                    Err(BandwidthControllerError::TicketbooksUnavailable) => break,
                    Err(BandwidthControllerError::TicketbookFetchFailed { .. }) => {}
                    other => panic!("unexpected readiness outcome: {other:?}"),
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("startup fetch must fail and settle");
    assert_eq!(calls.fetches().len(), 1, "exactly the startup fetch so far");

    await_stocked(&sender, types.clone())
        .await
        .expect("the fallback restock must make the type ready");

    assert_eq!(
        calls.fetches().len(),
        2,
        "exactly one restock fetch after the failed startup fetch"
    );
    cancel.cancel();
    let _ = task.await;
}
