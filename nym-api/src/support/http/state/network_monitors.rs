// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::AxumErrorResponse;
use crate::support::http::state::helpers::ChainSharedCacheWithTtl;
use crate::support::nyxd::Client;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{error, warn};

/// Per-orchestrator high-water marks of accepted submission timestamps, kept in-memory to provide
/// replay protection for the batch submission endpoints.
///
/// One map PER ENDPOINT, never one shared map. A single orchestrator identity signs every stream
/// it submits, and an endpoint rejects any batch whose timestamp is not strictly greater than that
/// signer's last accepted one, so two streams sharing one mark would reject each other
/// indefinitely - whichever posted second would always look replayed.
///
/// Held in memory only, so every mark resets to the process-online time on restart. The database
/// primary key is what actually guarantees idempotency; this only bounds replay within one process
/// lifetime.
#[derive(Clone)]
pub(crate) struct LastNMSubmissions {
    stress: Arc<RwLock<HashMap<ed25519::PublicKey, OffsetDateTime>>>,
    liveness: Arc<RwLock<HashMap<ed25519::PublicKey, OffsetDateTime>>>,
}

impl LastNMSubmissions {
    pub(crate) fn new() -> LastNMSubmissions {
        LastNMSubmissions {
            stress: Arc::new(Default::default()),
            liveness: Arc::new(Default::default()),
        }
    }

    /// Last accepted STRESS submission timestamp for a particular network monitor.
    pub(crate) async fn stress_submitted(&self, nm: ed25519::PublicKey) -> Option<OffsetDateTime> {
        self.stress.read().await.get(&nm).copied()
    }

    /// Record `timestamp` as `nm`'s most recent accepted STRESS submission.
    ///
    /// Callers are responsible for ensuring `timestamp` passes the monotonicity check against
    /// [`stress_submitted`][Self::stress_submitted] before calling this.
    pub(crate) async fn set_stress_submitted(
        &self,
        nm: ed25519::PublicKey,
        timestamp: OffsetDateTime,
    ) {
        self.stress.write().await.insert(nm, timestamp);
    }

    /// Last accepted LIVENESS submission timestamp for a particular network monitor.
    pub(crate) async fn liveness_submitted(
        &self,
        nm: ed25519::PublicKey,
    ) -> Option<OffsetDateTime> {
        self.liveness.read().await.get(&nm).copied()
    }

    /// Record `timestamp` as `nm`'s most recent accepted LIVENESS submission.
    ///
    /// Callers are responsible for ensuring `timestamp` passes the monotonicity check against
    /// [`liveness_submitted`][Self::liveness_submitted] before calling this.
    pub(crate) async fn set_liveness_submitted(
        &self,
        nm: ed25519::PublicKey,
        timestamp: OffsetDateTime,
    ) {
        self.liveness.write().await.insert(nm, timestamp);
    }
}

/// Snapshot of identity keys for network monitor orchestrators currently registered in the
/// network-monitors contract.
#[derive(Clone)]
pub(crate) struct KnownNetworkMonitors {
    known: HashSet<ed25519::PublicKey>,
}

impl KnownNetworkMonitors {
    pub(crate) fn contains(&self, key: &ed25519::PublicKey) -> bool {
        self.known.contains(key)
    }
}

/// TTL-gated cache over [`KnownNetworkMonitors`] so that every submission doesn't re-query the
/// network-monitors contract; refresh happens lazily on the first request after the TTL expires.
#[derive(Clone)]
pub(crate) struct NetworkMonitorsCache(ChainSharedCacheWithTtl<KnownNetworkMonitors>);

impl NetworkMonitorsCache {
    pub(crate) fn new(cache_ttl: Duration) -> Self {
        NetworkMonitorsCache(ChainSharedCacheWithTtl::new(cache_ttl))
    }

    /// Return the currently-cached set of known orchestrators, refreshing from chain if stale.
    pub(crate) async fn get_or_refresh(
        &self,
        client: &Client,
    ) -> Result<KnownNetworkMonitors, AxumErrorResponse> {
        self.0.get_or_refresh(client, refresh).await
    }

    /// Shortcut for "is this key in the current (possibly just-refreshed) orchestrator set?".
    pub(crate) async fn is_authorised(
        &self,
        nyxd_client: &Client,
        key: &ed25519::PublicKey,
    ) -> Result<bool, AxumErrorResponse> {
        Ok(self.get_or_refresh(nyxd_client).await?.known.contains(key))
    }
}

/// Fetch the orchestrator set from the network-monitors contract and decode each entry's identity
/// key. Orchestrators without an announced key, or with an unparseable one, are logged and
/// skipped - the rest still populate the cache so one bad entry doesn't take down submissions for
/// everyone.
async fn refresh(client: &Client) -> Result<KnownNetworkMonitors, NyxdError> {
    if let Err(err) = client.get_network_monitors_contract_address().await {
        warn!("network monitor contract address not set - can't accept any stress testing results");
        return Err(err);
    }

    let known_monitors = client.get_all_network_monitor_orchestrators().await?;
    let mut updated_monitors = HashSet::new();
    for monitor in known_monitors {
        let Some(public_key) = monitor.identity_key else {
            warn!("{} orchestrator is authorised but has not announced its public key - is the process running correctly?", monitor.address);
            continue;
        };
        let parsed = match ed25519::PublicKey::from_base58_string(&public_key) {
            Ok(key) => key,
            Err(err) => {
                error!("failed to parse public key for {}: {err}", monitor.address);
                continue;
            }
        };
        updated_monitors.insert(parsed);
    }
    Ok(KnownNetworkMonitors {
        known: updated_monitors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use time::macros::datetime;

    fn monitor() -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut OsRng).public_key()
    }

    /// The whole point of keeping a map per endpoint. Two decoys, one per component of what makes
    /// a mark distinct: the OTHER stream for the same signer, and the SAME stream for another
    /// signer. Without both, a lookup that ignored one of them would still pass.
    #[tokio::test]
    async fn a_mark_set_on_one_stream_is_invisible_to_the_other_and_to_other_signers() {
        let submissions = LastNMSubmissions::new();
        let (orchestrator, other_orchestrator) = (monitor(), monitor());
        let timestamp = datetime!(2026-09-03 12:00:00 UTC);

        submissions
            .set_stress_submitted(orchestrator, timestamp)
            .await;

        assert_eq!(
            submissions.stress_submitted(orchestrator).await,
            Some(timestamp)
        );
        // decoy 1: the other stream for this signer
        assert_eq!(submissions.liveness_submitted(orchestrator).await, None);
        // decoy 2: this stream for another signer
        assert_eq!(submissions.stress_submitted(other_orchestrator).await, None);
    }

    /// The failure this scoping exists to prevent: one orchestrator signs both streams, so with a
    /// shared mark the stream that posted second would look replayed forever. An out-of-order
    /// liveness timestamp must neither be gated by the stress mark nor move it.
    #[tokio::test]
    async fn interleaved_streams_from_one_signer_keep_independent_marks() {
        let submissions = LastNMSubmissions::new();
        let orchestrator = monitor();
        let later = datetime!(2026-09-03 12:00:00 UTC);
        let earlier = datetime!(2026-09-03 11:00:00 UTC);

        submissions.set_stress_submitted(orchestrator, later).await;
        submissions
            .set_liveness_submitted(orchestrator, earlier)
            .await;

        // each stream reads back its own timestamp, and the earlier liveness one did not clobber
        // the later stress one
        assert_eq!(
            submissions.stress_submitted(orchestrator).await,
            Some(later)
        );
        assert_eq!(
            submissions.liveness_submitted(orchestrator).await,
            Some(earlier)
        );
    }
}
