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
