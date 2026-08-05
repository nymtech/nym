// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Persistent per-gateway registration cache.
//!
//! Every fresh gateway registration spends one zk-nym ticket and grants a
//! bandwidth allowance keyed to the client's WireGuard public key — an
//! allowance the gateway keeps across disconnects. This cache persists what a
//! client must remember to come back to that peer instead of paying for a new
//! one: the client WireGuard private key and the gateway-returned
//! [`WireguardConfiguration`] (assigned addresses, gateway public key, PSK,
//! endpoint), keyed by (network name, gateway identity, hop role).
//!
//! The cache is deliberately not a validity oracle: entries are validated by
//! *use* (bounded tunnel establishment; see the smoldvpn
//! `Tunnel::await_established` API) and invalidated by the caller when a
//! cached peer no longer works. The only freshness logic here is hygiene: an
//! entry older than [`MAX_CACHE_AGE`] is treated as absent and pruned.
//!
//! Storage: `registrations.json` in the session data directory, written
//! atomically (temp file + rename) and created with owner-only permissions on
//! unix. It contains WireGuard **private keys** and PSKs — the same secret
//! sensitivity class as the credential store (`creds.db`) beside it.

use std::path::{Path, PathBuf};

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_registration_common::WireguardConfiguration;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{debug, info, warn};

use crate::gateway::WgRole;

/// File name of the cache inside the session data directory.
pub(crate) const CACHE_FILE_NAME: &str = "registrations.json";

/// Hygiene bound: entries older than this are treated as absent and pruned on
/// save. Purely to skip near-certainly-GC'd peers and bound file growth —
/// real validity is always established by use.
pub(crate) const MAX_CACHE_AGE: time::Duration = time::Duration::days(30);

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
struct CacheDocument {
    version: u32,
    entries: Vec<CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    /// Network name the registration belongs to (e.g. `mainnet`); entries are
    /// never reused across networks.
    network: String,
    /// bs58 ed25519 identity of the gateway.
    gateway: String,
    role: WgRole,
    /// bs58 x25519 client private key — the credential for the gateway-side
    /// peer and its remaining allowance.
    client_private_key: String,
    wg_config: WireguardConfiguration,
    /// Unix timestamp (seconds) of the registration.
    registered_at: i64,
}

/// A cache hit: everything hop assembly needs beyond gateway selection.
pub(crate) struct CachedRegistration {
    pub(crate) client_private_key: x25519::PrivateKey,
    pub(crate) wg_config: WireguardConfiguration,
}

/// The registration cache for one session (one network, one data directory).
pub(crate) struct RegistrationCache {
    path: PathBuf,
    network: String,
    doc: CacheDocument,
}

/// `WireguardConfiguration` does not implement `Clone` — and deriving it
/// upstream is NOT the fix: its `PresharedKey` is `Zeroize + ZeroizeOnDrop`
/// secret material that deliberately omits `Clone` so copies don't
/// proliferate. Round-trip through serde instead (it is
/// `Serialize + Deserialize`, which the cache already relies on for
/// persistence). Failure is treated as a cache miss by callers — never fatal.
fn clone_cfg(cfg: &WireguardConfiguration) -> Option<WireguardConfiguration> {
    match serde_json::to_value(cfg).and_then(serde_json::from_value) {
        Ok(cloned) => Some(cloned),
        Err(e) => {
            warn!("failed to round-trip a cached wireguard configuration: {e}");
            None
        }
    }
}

fn now_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn expired(registered_at: i64) -> bool {
    let age = now_unix().saturating_sub(registered_at);
    age > MAX_CACHE_AGE.whole_seconds()
}

impl RegistrationCache {
    /// Load the cache for `network` from `data_path`. An absent, unreadable,
    /// or unparseable file degrades to an empty cache (warn at most) — the
    /// cache must never fail a session on its own account.
    pub(crate) fn load(data_path: &Path, network: impl Into<String>) -> Self {
        let path = data_path.join(CACHE_FILE_NAME);
        let doc = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<CacheDocument>(&bytes) {
                Ok(doc) if doc.version == CURRENT_VERSION => doc,
                Ok(doc) => {
                    warn!(
                        "ignoring registration cache {} with unknown version {}",
                        path.display(),
                        doc.version
                    );
                    CacheDocument::default()
                }
                Err(e) => {
                    warn!(
                        "ignoring unparseable registration cache {}: {e}",
                        path.display()
                    );
                    CacheDocument::default()
                }
            },
            // absent file: the normal first-run case
            Err(_) => CacheDocument::default(),
        };
        RegistrationCache {
            path,
            network: network.into(),
            doc,
        }
    }

    /// Look up a usable cached registration for (gateway, role) on this
    /// network. Expired entries are treated as absent.
    pub(crate) fn lookup(
        &self,
        gateway: &ed25519::PublicKey,
        role: WgRole,
    ) -> Option<CachedRegistration> {
        let gateway = gateway.to_base58_string();
        let entry = self.doc.entries.iter().find(|e| {
            e.network == self.network
                && e.role == role
                && e.gateway == gateway
                && !expired(e.registered_at)
        })?;
        let client_private_key =
            match x25519::PrivateKey::from_base58_string(&entry.client_private_key) {
                Ok(key) => key,
                Err(e) => {
                    warn!(
                        "cached registration for {gateway} ({role:?}) has an unparseable key: {e}"
                    );
                    return None;
                }
            };
        let wg_config = clone_cfg(&entry.wg_config)?;
        Some(CachedRegistration {
            client_private_key,
            wg_config,
        })
    }

    /// Record a fresh, successful registration (replacing any previous entry
    /// for the same key) and persist. A persistence failure is logged and
    /// swallowed: worst case is one extra future spend, never a lost
    /// registration.
    pub(crate) fn insert(
        &mut self,
        gateway: &ed25519::PublicKey,
        role: WgRole,
        client_private_key: &x25519::PrivateKey,
        wg_config: &WireguardConfiguration,
    ) {
        let Some(wg_config) = clone_cfg(wg_config) else {
            return;
        };
        let gateway = gateway.to_base58_string();
        self.doc
            .entries
            .retain(|e| !(e.network == self.network && e.role == role && e.gateway == gateway));
        self.doc.entries.push(CacheEntry {
            network: self.network.clone(),
            gateway,
            role,
            client_private_key: client_private_key.to_base58_string(),
            wg_config,
            registered_at: now_unix(),
        });
        self.save();
    }

    /// Remove the entry for (gateway, role) on this network, persisting the
    /// removal. Absent entries are a no-op. Returns whether an entry existed.
    pub(crate) fn remove(&mut self, gateway: &ed25519::PublicKey, role: WgRole) -> bool {
        let gateway = gateway.to_base58_string();
        let before = self.doc.entries.len();
        self.doc
            .entries
            .retain(|e| !(e.network == self.network && e.role == role && e.gateway == gateway));
        let removed = self.doc.entries.len() != before;
        if removed {
            info!("invalidated cached registration for {gateway} ({role:?})");
            self.save();
        }
        removed
    }

    /// Persist the cache: prune expired entries, then write atomically
    /// (temp file + rename), creating the file owner-only on unix.
    fn save(&mut self) {
        self.doc.entries.retain(|e| !expired(e.registered_at));
        self.doc.version = CURRENT_VERSION;
        let bytes = match serde_json::to_vec_pretty(&self.doc) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("failed to serialize the registration cache: {e}");
                return;
            }
        };
        let tmp = self.path.with_extension("json.tmp");
        if let Err(e) = write_owner_only(&tmp, &bytes) {
            warn!("failed to write registration cache {}: {e}", tmp.display());
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, &self.path) {
            warn!(
                "failed to move registration cache into place at {}: {e}",
                self.path.display()
            );
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        debug!(
            "registration cache saved ({} entries)",
            self.doc.entries.len()
        );
    }
}

/// Write `bytes` to `path`, creating the file with owner-only permissions on
/// unix (best-effort elsewhere).
fn write_owner_only(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        f.sync_all()
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    fn gateway_key(seed: u8) -> ed25519::PublicKey {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::from_seed([seed; 32]);
        *ed25519::KeyPair::new(&mut rng).public_key()
    }

    fn client_key(seed: u8) -> x25519::PrivateKey {
        x25519::PrivateKey::from_secret([seed; 32])
    }

    fn wg_config(port: u16) -> WireguardConfiguration {
        WireguardConfiguration {
            public_key: x25519::PrivateKey::from_secret([9; 32]).public_key(),
            psk: None,
            endpoint: SocketAddr::from(([192, 0, 2, 1], port)),
            private_ipv4: Ipv4Addr::new(10, 1, 2, 3),
            private_ipv6: Ipv6Addr::LOCALHOST,
        }
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nym-reg-cache-test-{}-{:x}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 4.1: an inserted registration is served back by a freshly loaded
    /// instance (round-trip through disk), with key and config intact.
    #[test]
    fn round_trips_through_disk() {
        let dir = tempdir();
        let gw = gateway_key(1);

        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw, WgRole::Entry, &client_key(7), &wg_config(51822));

        let reloaded = RegistrationCache::load(&dir, "mainnet");
        let hit = reloaded.lookup(&gw, WgRole::Entry).expect("cache hit");
        assert_eq!(hit.client_private_key.to_bytes(), [7; 32]);
        assert_eq!(hit.wg_config, wg_config(51822));
    }

    /// 4.1: entries are keyed by (gateway, role) — the wrong role or an
    /// unknown gateway misses.
    #[test]
    fn keyed_by_gateway_and_role() {
        let dir = tempdir();
        let gw = gateway_key(1);
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw, WgRole::Entry, &client_key(7), &wg_config(51822));

        assert!(cache.lookup(&gw, WgRole::Exit).is_none());
        assert!(cache.lookup(&gateway_key(2), WgRole::Entry).is_none());
        assert!(cache.lookup(&gw, WgRole::Entry).is_some());
    }

    /// 4.1: entries recorded under another network are never reused.
    #[test]
    fn network_isolation() {
        let dir = tempdir();
        let gw = gateway_key(1);
        let mut mainnet = RegistrationCache::load(&dir, "mainnet");
        mainnet.insert(&gw, WgRole::Entry, &client_key(7), &wg_config(51822));

        let sandbox = RegistrationCache::load(&dir, "sandbox");
        assert!(sandbox.lookup(&gw, WgRole::Entry).is_none());
        // and the mainnet view still sees it
        let mainnet = RegistrationCache::load(&dir, "mainnet");
        assert!(mainnet.lookup(&gw, WgRole::Entry).is_some());
    }

    /// 4.1: invalidation removes exactly the keyed entry, persistently.
    #[test]
    fn invalidation_removes_exactly_the_keyed_entry() {
        let dir = tempdir();
        let gw1 = gateway_key(1);
        let gw2 = gateway_key(2);
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw1, WgRole::Entry, &client_key(7), &wg_config(51822));
        cache.insert(&gw2, WgRole::Exit, &client_key(8), &wg_config(51823));

        assert!(cache.remove(&gw1, WgRole::Entry));
        assert!(
            !cache.remove(&gw1, WgRole::Entry),
            "second remove is a no-op"
        );

        let reloaded = RegistrationCache::load(&dir, "mainnet");
        assert!(reloaded.lookup(&gw1, WgRole::Entry).is_none());
        assert!(reloaded.lookup(&gw2, WgRole::Exit).is_some());
    }

    /// 4.1: replacing a registration for the same key keeps only the newest.
    #[test]
    fn insert_replaces_previous_entry() {
        let dir = tempdir();
        let gw = gateway_key(1);
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw, WgRole::Entry, &client_key(7), &wg_config(51822));
        cache.insert(&gw, WgRole::Entry, &client_key(8), &wg_config(51823));

        let hit = cache.lookup(&gw, WgRole::Entry).expect("cache hit");
        assert_eq!(hit.client_private_key.to_bytes(), [8; 32]);
        assert_eq!(cache.doc.entries.len(), 1);
    }

    /// 4.1: corrupt or absent cache files degrade to an empty cache.
    #[test]
    fn corrupt_or_absent_file_degrades_to_empty() {
        let dir = tempdir();
        // absent
        let cache = RegistrationCache::load(&dir, "mainnet");
        assert!(cache.lookup(&gateway_key(1), WgRole::Entry).is_none());
        // corrupt
        std::fs::write(dir.join(CACHE_FILE_NAME), b"{ not json").unwrap();
        let cache = RegistrationCache::load(&dir, "mainnet");
        assert!(cache.lookup(&gateway_key(1), WgRole::Entry).is_none());
        // and a save from the degraded state produces a valid file again
        let mut cache = cache;
        cache.insert(
            &gateway_key(1),
            WgRole::Entry,
            &client_key(7),
            &wg_config(1),
        );
        assert!(RegistrationCache::load(&dir, "mainnet")
            .lookup(&gateway_key(1), WgRole::Entry)
            .is_some());
    }

    /// 4.1: entries older than MAX_CACHE_AGE are treated as absent and pruned.
    #[test]
    fn expired_entries_are_absent_and_pruned() {
        let dir = tempdir();
        let gw = gateway_key(1);
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(&gw, WgRole::Entry, &client_key(7), &wg_config(51822));
        // age the entry past the bound
        cache.doc.entries[0].registered_at = now_unix() - MAX_CACHE_AGE.whole_seconds() - 1;
        assert!(cache.lookup(&gw, WgRole::Entry).is_none());
        cache.save();
        assert!(cache.doc.entries.is_empty(), "pruned on save");
    }

    /// 4.1: the cache file is created owner-only on unix.
    #[cfg(unix)]
    #[test]
    fn cache_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir();
        let mut cache = RegistrationCache::load(&dir, "mainnet");
        cache.insert(
            &gateway_key(1),
            WgRole::Entry,
            &client_key(7),
            &wg_config(1),
        );
        let mode = std::fs::metadata(dir.join(CACHE_FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
