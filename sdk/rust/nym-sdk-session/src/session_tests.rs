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
use std::sync::atomic::{AtomicUsize, Ordering};

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
            two_hop: true,
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
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

// `BandwidthController`, `BandwidthControllerConfig`, `CredentialFetcher`, `ShutdownToken`,
// `TicketType`, `Arc`, `AtomicBool`, `Ordering` and `wireguard_ticket_types` come in via
// `use super::*`.
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

/// Call counters shared across every `RecordingFetcher` a factory builds.
#[derive(Default)]
struct FetcherCalls {
    /// Distinct fetcher instances the factory built (one per install).
    builds: AtomicUsize,
    fetch_ticketbooks: AtomicUsize,
    cleanups: AtomicUsize,
}

impl FetcherCalls {
    fn builds(&self) -> usize {
        self.builds.load(Ordering::SeqCst)
    }
    fn fetches(&self) -> usize {
        self.fetch_ticketbooks.load(Ordering::SeqCst)
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

/// A fetcher that records its calls and never mints real credentials. `fail` makes each ticketbook
/// fetch error; otherwise it returns an empty batch (so readiness never resolves to `Ready` — enough
/// to exercise install/fetch/removal, not the success path). The public-data methods are never
/// driven (no ticketbook is ever stored) and simply error if they somehow were.
///
/// It models the real `NyxdCredentialFetcher`'s `cleanup` (which closes the recovery store): once
/// cleaned up, a further `fetch_ticketbooks` fails. So a test that reused a removed instance —
/// exactly the bug the fresh-fetcher-per-install change fixes — would fail instead of silently
/// passing on a broken double (per review feedback).
struct RecordingFetcher {
    calls: Arc<FetcherCalls>,
    fail: bool,
    cleaned: AtomicBool,
}

impl RecordingFetcher {
    fn new(calls: Arc<FetcherCalls>, fail: bool) -> Self {
        Self {
            calls,
            fail,
            cleaned: AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl CredentialFetcher for RecordingFetcher {
    async fn fetch_ticketbooks(
        &self,
        _ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        // A cleaned-up fetcher has had its recovery store closed and can no longer fetch.
        if self.cleaned.load(Ordering::SeqCst) {
            return Err(RecordedFailure.into());
        }
        self.calls.fetch_ticketbooks.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err(RecordedFailure.into())
        } else {
            Ok(Vec::new())
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
        _epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, CredentialFetcherError> {
        Err(RecordedFailure.into())
    }
    async fn fetch_coin_index_signatures(
        &self,
        _epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        Err(RecordedFailure.into())
    }
    async fn fetch_expiration_date_signatures(
        &self,
        _expiration_date: Date,
        _epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        Err(RecordedFailure.into())
    }
}

/// Spawn a real controller (managed = WireGuard types) and wrap it in an `OwnedController` whose
/// factory builds a FRESH recording fetcher per install — mirroring what `spawn_controller` builds,
/// minus the chain client. Every built instance shares `calls`, so the counters aggregate across
/// installs while `builds` tracks how many distinct instances were made.
fn spawn_owned(
    calls: Arc<FetcherCalls>,
    fail: bool,
    auto_topup: bool,
    cancel: &CancellationToken,
) -> OwnedController {
    let storage = initialise_ephemeral_storage();
    let config = BandwidthControllerConfig {
        managed_ticket_types: needed_ticket_types(true),
        ..Default::default()
    };
    let controller = BandwidthController::new(storage).with_config(config);
    let sender = controller.get_request_sender();
    let shutdown = ShutdownToken::new_from_tokio_token(cancel.clone());
    let task = tokio::spawn(async move { controller.run(shutdown).await });

    let make_fetcher: FetcherFactory = {
        let calls = calls.clone();
        Arc::new(move || {
            let calls = calls.clone();
            Box::pin(async move {
                calls.builds.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::new(RecordingFetcher::new(calls, fail)) as Arc<dyn CredentialFetcher>)
            })
        })
    };
    OwnedController {
        sender,
        task,
        make_fetcher,
        fetcher_installed: AtomicBool::new(false),
        provision_lock: tokio::sync::Mutex::new(()),
        auto_topup,
    }
}

/// One-shot mode: provisioning installs the fetcher (which triggers a fetch of the managed
/// WireGuard type) and removes it afterwards, so the controller is left with no fetcher and makes no
/// background deposit.
#[tokio::test]
async fn default_mode_installs_fetcher_then_removes_it() {
    let calls = Arc::new(FetcherCalls::default());
    let cancel = CancellationToken::new();
    let owned = spawn_owned(calls.clone(), false, false, &cancel);

    // (Returns an error because the mock never mints a real ticketbook; we assert the lifecycle.)
    let _ = owned
        .ensure(vec![TicketType::V1WireguardEntry], &cancel)
        .await;

    assert_eq!(
        calls.builds(),
        1,
        "provisioning must build exactly one fetcher to install"
    );
    assert!(
        calls.fetches() >= 1,
        "installing the fetcher must trigger a restock fetch of the managed type"
    );
    assert!(
        !owned.fetcher_installed.load(Ordering::SeqCst),
        "one-shot mode must remove the fetcher after provisioning"
    );
    assert!(
        calls.cleanups() >= 1,
        "removing the fetcher cleans it up (proving it was actually unset)"
    );

    cancel.cancel();
    let _ = owned.task.await;
}

/// One-shot mode builds a FRESH fetcher for every provision: removal `cleanup`s (closes the recovery
/// store of) the previous instance, so it can't be reinstalled. Two provisions ⇒ two distinct builds
/// and (at least) two cleanups. This is the regression guard for the fetcher-reuse bug the
/// fresh-fetcher-per-install change fixes.
#[tokio::test]
async fn one_shot_builds_a_fresh_fetcher_per_provision() {
    let calls = Arc::new(FetcherCalls::default());
    let cancel = CancellationToken::new();
    let owned = spawn_owned(calls.clone(), false, false, &cancel);

    let _ = owned
        .ensure(vec![TicketType::V1WireguardEntry], &cancel)
        .await;
    let _ = owned
        .ensure(vec![TicketType::V1WireguardEntry], &cancel)
        .await;

    assert_eq!(
        calls.builds(),
        2,
        "each one-shot provision must build its own fresh fetcher, never reuse a cleaned-up one"
    );
    assert!(
        calls.cleanups() >= 2,
        "each provision must remove (clean up) the fetcher it installed"
    );
    assert!(
        !owned.fetcher_installed.load(Ordering::SeqCst),
        "one-shot mode leaves no fetcher installed"
    );

    cancel.cancel();
    let _ = owned.task.await;
}

/// One-shot mode removes the fetcher even when provisioning fails, so a failed provision never
/// leaves background restock enabled.
#[tokio::test]
async fn default_mode_removes_fetcher_even_on_failure() {
    let calls = Arc::new(FetcherCalls::default());
    let cancel = CancellationToken::new();
    let owned = spawn_owned(calls.clone(), true, false, &cancel);

    let res = owned
        .ensure(vec![TicketType::V1WireguardEntry], &cancel)
        .await;

    assert!(
        res.is_err(),
        "a failing fetch must surface as a provisioning error"
    );
    assert!(
        !owned.fetcher_installed.load(Ordering::SeqCst),
        "the fetcher must be removed on the failure path too"
    );
    assert!(
        calls.cleanups() >= 1,
        "removal must clean up the fetcher even after a failed provision"
    );

    cancel.cancel();
    let _ = owned.task.await;
}

/// Automatic top-up mode leaves the fetcher installed after provisioning (so the controller's sweep
/// restocks per policy), and a subsequent provisioning call skips the re-install while it is present.
#[tokio::test]
async fn auto_topup_keeps_fetcher_installed_and_skips_reinstall() {
    let calls = Arc::new(FetcherCalls::default());
    let cancel = CancellationToken::new();
    let owned = spawn_owned(calls.clone(), false, true, &cancel);

    let _ = owned
        .ensure(vec![TicketType::V1WireguardEntry], &cancel)
        .await;

    assert!(
        owned.fetcher_installed.load(Ordering::SeqCst),
        "automatic top-up mode must keep the fetcher installed"
    );

    // A follow-up install request is skipped while the fetcher is present (no new fetch).
    let fetches_before = calls.fetches();
    owned
        .set_fetcher_if_absent()
        .await
        .expect("skip must not error");
    assert_eq!(
        calls.fetches(),
        fetches_before,
        "an already-installed fetcher must not be re-installed (no extra fetch)"
    );

    cancel.cancel();
    let _ = owned.task.await;
}
