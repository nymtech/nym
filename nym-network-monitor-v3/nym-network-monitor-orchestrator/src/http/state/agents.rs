// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::orchestrator::prometheus::{PROMETHEUS_METRICS, PrometheusMetric};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_monitor_orchestrator_requests::models::AgentMixAddresses;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::{error, warn};

/// Thread-safe cache of all agents known to this orchestrator, keyed by the agent's IPv4 mixnet
/// address. Used to short-circuit the contract txs for already-announced agents.
#[derive(Clone, Default)]
pub(crate) struct KnownAgents {
    inner: Arc<Mutex<KnownAgentsInner>>,
}

impl KnownAgents {
    /// Looks up the agent making a request by the address pair it announced, and records that it
    /// requested work. Returns `None` if nothing is registered under its IPv4 address, or if the
    /// registered entry belongs to a different IPv6 address, i.e. this is not the agent we
    /// authorised, in which case nothing is recorded.
    ///
    /// The stamp is what `agents_seen_recently` counts, and it is taken here because this is the
    /// only place the orchestrator resolves a caller it already knows.
    pub(crate) async fn resolve_requesting_agent(
        &self,
        addresses: AgentMixAddresses,
    ) -> Option<KnownAgent> {
        let mut guard = self.inner.lock().await;
        let agent = guard.agents.get_mut(&addresses.v4)?;

        if agent.mix_v6 != addresses.v6 {
            return None;
        }

        agent.last_work_request_at = Some(OffsetDateTime::now_utc());
        Some(*agent)
    }

    /// How many agents have requested work within `window`.
    ///
    /// Counted rather than derived from the cache size: the cache is seeded from every agent ever
    /// authorised and is never pruned, so its size is addresses ever seen rather than agents
    /// currently working. An agent that restarted on a new address leaves a second entry behind,
    /// which falls out of this count as soon as it stops requesting work.
    pub(crate) async fn seen_within(&self, window: Duration) -> i64 {
        let cutoff = OffsetDateTime::now_utc() - window;
        let guard = self.inner.lock().await;

        guard
            .agents
            .values()
            .filter(|agent| agent.last_work_request_at.is_some_and(|at| at >= cutoff))
            .count() as i64
    }

    /// Records an announcement from the agent at `addresses`. The cache entry is upserted: a
    /// missing entry is inserted, and if the cached noise key, IPv6 address or ed25519 identity
    /// differs from the announced one it is overwritten and the agent is treated as not-yet-announced
    /// so the caller re-runs the contract txs with the new details.
    ///
    /// Returns the current `announced` flag: `true` means the agent was already announced to the
    /// contract and the caller should skip the contract txs; `false` means the caller should submit
    /// them and call [`Self::mark_announced`] on success.
    pub(crate) async fn try_announce_agent(
        &self,
        addresses: AgentMixAddresses,
        noise_key: x25519::PublicKey,
        ed25519_identity: ed25519::PublicKey,
    ) -> bool {
        let mut guard = self.inner.lock().await;

        match guard.agents.entry(addresses.v4) {
            Entry::Occupied(mut entry) => {
                let agent = entry.get_mut();

                let details_diverged = agent.noise_key != noise_key || agent.mix_v6 != addresses.v6;
                let identity_diverged = agent.ed25519_identity != ed25519_identity;

                if !details_diverged && !identity_diverged {
                    return agent.announced;
                }

                // the addresses, the noise key and the identity are all meant to be stable for the
                // lifetime of an agent, so this is either a re-provisioned agent reusing its IPv4
                // address or a live one whose configuration changed. we can't distinguish those
                // (announcements are bearer-token authenticated, not key authenticated), so the
                // announcement is accepted - but a superseded IPv6 address stays authorised in the
                // contract, which is worth knowing about.
                //
                // both kinds of divergence share the one counter: the identity is derived from the
                // noise key, so it can only change when the noise key does, and telling the two
                // apart is a job for the logs below rather than for a second series that would read
                // flat zero outside a change to the derivation itself.
                PROMETHEUS_METRICS.inc(PrometheusMetric::AgentDetailsChanged);

                if details_diverged {
                    warn!(
                        "agent at {} announced details differing from the cached ones (cached: {} / {}, announced: {} / {}) - re-announcing it to the contract",
                        addresses.v4,
                        agent.mix_v6,
                        agent.noise_key.to_base58_string(),
                        addresses.v6,
                        noise_key.to_base58_string(),
                    );
                }

                if identity_diverged {
                    warn!(
                        "agent at {} announced an ed25519 identity differing from the cached one (cached: {}, announced: {ed25519_identity}) - re-announcing it to the contract",
                        addresses.v4, agent.ed25519_identity,
                    );
                }

                agent.mix_v6 = addresses.v6;
                agent.noise_key = noise_key;
                agent.ed25519_identity = ed25519_identity;
                agent.announced = false;
            }
            Entry::Vacant(entry) => {
                entry.insert(KnownAgent {
                    mix_v6: addresses.v6,
                    last_work_request_at: None,
                    noise_key,
                    ed25519_identity,
                    announced: false,
                });
            }
        }

        false
    }

    /// Marks the agent at `addresses` as announced, unless a later announcement has since replaced
    /// its keys. Should be called once every contract transaction for that agent has succeeded.
    pub(crate) async fn mark_announced(
        &self,
        addresses: AgentMixAddresses,
        noise_key: x25519::PublicKey,
        ed25519_identity: ed25519::PublicKey,
    ) {
        let mut guard = self.inner.lock().await;
        if let Some(agent) = guard.agents.get_mut(&addresses.v4)
            && agent.mix_v6 == addresses.v6
            && agent.noise_key == noise_key
            && agent.ed25519_identity == ed25519_identity
        {
            agent.announced = true;
        }
    }
}

/// Rebuilds the agent cache from on-chain data. Used at orchestrator startup to
/// restore state for agents that were authorised before a restart.
///
/// The contract holds one entry per socket address and nothing ties together the two entries
/// belonging to a single agent, so the pairs are recovered by grouping on the noise key, which is
/// unique per agent. Records that don't form exactly one IPv4/IPv6 pair carrying one identity are
/// dropped: they are either authorisations predating the IPv6 announcement or the identity key, or
/// leftovers from an agent that has since changed one of its addresses. Dropping them is safe
/// because this cache exists purely to skip redundant contract transactions - agents always announce
/// before requesting work, which re-creates the entry at the cost of one extra transaction, and the
/// contract's upsert fills in whatever the stale entry was missing.
impl TryFrom<Vec<AuthorisedNetworkMonitor>> for KnownAgents {
    type Error = anyhow::Error;

    fn try_from(agents: Vec<AuthorisedNetworkMonitor>) -> Result<Self, Self::Error> {
        let mut by_noise_key: HashMap<String, Vec<AuthorisedNetworkMonitor>> = HashMap::new();
        for agent in agents {
            by_noise_key
                .entry(agent.bs58_x25519_noise.clone())
                .or_default()
                .push(agent);
        }

        let mut agents_map = HashMap::new();
        for (bs58_noise_key, entries) in by_noise_key {
            let addresses: Vec<_> = entries.iter().map(|entry| entry.mixnet_address).collect();
            let (v4_entries, v6_entries): (Vec<_>, Vec<_>) = entries
                .into_iter()
                .partition(|entry| entry.mixnet_address.is_ipv4());

            let ([v4_entry], [v6_entry]) = (v4_entries.as_slice(), v6_entries.as_slice()) else {
                error!(
                    "the agent using noise key {bs58_noise_key} has {} authorised address(es) on chain ({addresses:?}) rather than a single ipv4/ipv6 pair - ignoring it until it re-announces itself",
                    addresses.len()
                );
                continue;
            };

            // an agent announces one identity under both of its addresses, so anything else is a
            // half-written pair: an entry authorised before the field existed, or one address left
            // behind by an agent that has since rotated its noise key
            let (Some(v4_identity), Some(v6_identity)) = (
                &v4_entry.bs58_ed25519_identity,
                &v6_entry.bs58_ed25519_identity,
            ) else {
                error!(
                    "the agent using noise key {bs58_noise_key} has an authorised address ({addresses:?}) with no announced ed25519 identity - ignoring it until it re-announces itself"
                );
                continue;
            };

            if v4_identity != v6_identity {
                error!(
                    "the agent using noise key {bs58_noise_key} announced different ed25519 identities under its two addresses ({v4_identity} / {v6_identity}) - ignoring it until it re-announces itself"
                );
                continue;
            }

            let noise_key = x25519::PublicKey::from_base58_string(&bs58_noise_key)?;
            let ed25519_identity = ed25519::PublicKey::from_base58_string(v4_identity)?;
            agents_map.insert(
                v4_entry.mixnet_address,
                KnownAgent {
                    mix_v6: v6_entry.mixnet_address,
                    last_work_request_at: None,
                    noise_key,
                    ed25519_identity,
                    announced: true,
                },
            );
        }

        Ok(KnownAgents {
            inner: Arc::new(Mutex::new(KnownAgentsInner { agents: agents_map })),
        })
    }
}

/// Inner state behind the [`KnownAgents`] mutex.
#[derive(Default)]
struct KnownAgentsInner {
    /// Map from the agent's IPv4 mixnet address to its cached state. The port is part of the key
    /// because several agents may share a host IP.
    agents: HashMap<SocketAddr, KnownAgent>,
}

/// Cached state of a single known agent, i.e. of the pair of contract authorisations it holds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnownAgent {
    /// The IPv6 mixnet address announced alongside the IPv4 address this entry is keyed by.
    pub(crate) mix_v6: SocketAddr,

    /// When this agent last requested work, `None` until it does. What `agents_seen_recently`
    /// counts against its window.
    pub(crate) last_work_request_at: Option<OffsetDateTime>,
    pub(crate) noise_key: x25519::PublicKey,

    /// The ed25519 identity this agent presents when opening a gateway client session.
    pub(crate) ed25519_identity: ed25519::PublicKey,

    /// Whether this agent has been successfully registered in the smart contract, under both of
    /// its addresses. Set to `true` when restored from the chain at startup, or after a successful
    /// `/announce` contract transaction.
    pub(crate) announced: bool,
}

#[cfg(test)]
impl KnownAgents {
    /// Backdates when the agent at `addresses` last requested work, standing in for one that has
    /// gone quiet.
    pub(crate) async fn backdate_last_work_request(
        &self,
        addresses: AgentMixAddresses,
        ago: Duration,
    ) {
        let mut guard = self.inner.lock().await;
        if let Some(agent) = guard.agents.get_mut(&addresses.v4) {
            agent.last_work_request_at = Some(OffsetDateTime::now_utc() - ago);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr, Timestamp};
    use nym_test_utils::helpers::seeded_rng;

    fn noise_key(seed: u8) -> x25519::PublicKey {
        x25519::PublicKey::from(&x25519::PrivateKey::new(&mut seeded_rng([seed; 32])))
    }

    fn identity(seed: u8) -> ed25519::PublicKey {
        *ed25519::KeyPair::new(&mut seeded_rng([seed; 32])).public_key()
    }

    fn addresses(port: u16) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: format!("1.1.1.1:{port}").parse().unwrap(),
            v6: format!("[aaaa::{port}]:{port}").parse().unwrap(),
        }
    }

    fn authorisation(
        address: SocketAddr,
        noise_key: x25519::PublicKey,
        identity: Option<ed25519::PublicKey>,
    ) -> AuthorisedNetworkMonitor {
        AuthorisedNetworkMonitor {
            mixnet_address: address,
            authorised_by: Addr::unchecked("n1foomp"),
            authorised_at: Timestamp::from_seconds(42),
            bs58_x25519_noise: noise_key.to_base58_string(),
            noise_version: 1,
            bs58_ed25519_identity: identity.map(|key| key.to_base58_string()),
        }
    }

    #[tokio::test]
    async fn agent_is_not_announced_until_marked() {
        let agents = KnownAgents::default();
        let addresses = addresses(1789);
        let key = noise_key(1);

        assert!(!agents.try_announce_agent(addresses, key, identity(1)).await);
        assert!(
            !agents
                .resolve_requesting_agent(addresses)
                .await
                .unwrap()
                .announced
        );

        agents.mark_announced(addresses, key, identity(1)).await;
        assert!(
            agents
                .resolve_requesting_agent(addresses)
                .await
                .unwrap()
                .announced
        );

        // a repeated announcement is now a no-op, so the caller skips the contract txs
        assert!(agents.try_announce_agent(addresses, key, identity(1)).await);
    }

    // agents deployed on the same host share its ipv4 address and are only told apart by the port,
    // so an announcement from one must not disturb the other
    #[tokio::test]
    async fn agents_sharing_a_host_ip_are_tracked_separately() {
        let agents = KnownAgents::default();
        let first = addresses(1789);
        let second = addresses(1790);
        assert_eq!(first.v4.ip(), second.v4.ip());

        agents
            .try_announce_agent(first, noise_key(1), identity(1))
            .await;
        agents
            .mark_announced(first, noise_key(1), identity(1))
            .await;
        agents
            .try_announce_agent(second, noise_key(2), identity(2))
            .await;

        assert!(
            agents
                .resolve_requesting_agent(first)
                .await
                .unwrap()
                .announced
        );
        assert!(
            !agents
                .resolve_requesting_agent(second)
                .await
                .unwrap()
                .announced
        );
    }

    // The cache is seeded from every agent ever authorised and is never pruned, so the working
    // fleet has to be counted from recent activity. An agent that restarted on a new address leaves
    // its old entry behind, and that entry must stop being counted rather than inflating the fleet
    // with every restart.
    #[tokio::test]
    async fn only_agents_that_requested_work_within_the_window_are_counted() {
        let agents = KnownAgents::default();
        let abandoned = addresses(1789);
        let current = addresses(1790);

        agents
            .try_announce_agent(abandoned, noise_key(1), identity(1))
            .await;
        agents
            .try_announce_agent(current, noise_key(2), identity(2))
            .await;

        // announcing is not asking for work
        assert_eq!(0, agents.seen_within(Duration::from_secs(60 * 60)).await);

        agents.resolve_requesting_agent(abandoned).await.unwrap();
        agents.resolve_requesting_agent(current).await.unwrap();
        agents
            .backdate_last_work_request(abandoned, Duration::from_secs(30 * 60))
            .await;

        assert_eq!(2, agents.seen_within(Duration::from_secs(60 * 60)).await);
        assert_eq!(1, agents.seen_within(Duration::from_secs(15 * 60)).await);
    }

    // a restart restores every agent ever authorised, so counting them before they ask for work
    // would inflate the fleet for a whole window and then read as a loss
    #[tokio::test]
    async fn restored_agents_are_not_counted_until_they_request_work() {
        let agent = addresses(1789);
        let restored = KnownAgents::try_from(vec![
            authorisation(agent.v4, noise_key(1), Some(identity(1))),
            authorisation(agent.v6, noise_key(1), Some(identity(1))),
        ])
        .unwrap();

        assert_eq!(0, restored.seen_within(Duration::from_secs(60 * 60)).await);

        restored.resolve_requesting_agent(agent).await.unwrap();
        assert_eq!(1, restored.seen_within(Duration::from_secs(60 * 60)).await);
    }

    // the entry belongs to the announced pair, not to the ipv4 address alone
    #[tokio::test]
    async fn lookup_with_a_different_v6_address_finds_nothing() {
        let agents = KnownAgents::default();
        let announced = addresses(1789);
        let key = noise_key(1);

        agents.try_announce_agent(announced, key, identity(1)).await;
        agents
            .mark_announced(announced, noise_key(1), identity(1))
            .await;

        let other_v6 = AgentMixAddresses {
            v6: "[bbbb::1]:1789".parse().unwrap(),
            ..announced
        };
        assert!(agents.resolve_requesting_agent(other_v6).await.is_none());
    }

    #[tokio::test]
    async fn changed_details_require_a_re_announcement() {
        for changed in [
            AgentMixAddresses {
                v6: "[bbbb::1]:1789".parse().unwrap(),
                ..addresses(1789)
            },
            addresses(1789),
        ] {
            let agents = KnownAgents::default();
            let announced = addresses(1789);
            agents
                .try_announce_agent(announced, noise_key(1), identity(1))
                .await;
            agents
                .mark_announced(announced, noise_key(1), identity(1))
                .await;

            // either the v6 address or the noise key differs from what we have cached
            let key = if changed.v6 == announced.v6 {
                noise_key(2)
            } else {
                noise_key(1)
            };
            assert!(!agents.try_announce_agent(changed, key, identity(1)).await);

            let agent = agents.resolve_requesting_agent(changed).await.unwrap();
            assert!(!agent.announced);
            assert_eq!(agent.mix_v6, changed.v6);
            assert_eq!(agent.noise_key, key);
        }
    }

    // the identity is what a gateway keys the unmetered monitor session on, so an announcement
    // carrying a new one has to reach the contract even though every other detail is unchanged.
    // clearing the announced flag is what makes the caller re-authorise BOTH addresses
    #[tokio::test]
    async fn a_changed_identity_alone_requires_a_re_announcement() {
        let agents = KnownAgents::default();
        let announced = addresses(1789);

        agents
            .try_announce_agent(announced, noise_key(1), identity(1))
            .await;
        agents
            .mark_announced(announced, noise_key(1), identity(1))
            .await;

        assert!(
            !agents
                .try_announce_agent(announced, noise_key(1), identity(2))
                .await
        );

        let agent = agents.resolve_requesting_agent(announced).await.unwrap();
        assert!(!agent.announced);
        assert_eq!(agent.ed25519_identity, identity(2));
    }

    // The contract transaction of an earlier announcement can finish after a later one replaced
    // the agent's keys. It carried the old keys, so it must not mark the new ones as announced.
    #[tokio::test]
    async fn a_superseded_announcement_does_not_mark_the_new_keys_announced() {
        let agents = KnownAgents::default();
        let addresses = addresses(1789);

        agents
            .try_announce_agent(addresses, noise_key(1), identity(1))
            .await;
        agents
            .try_announce_agent(addresses, noise_key(2), identity(2))
            .await;
        agents
            .mark_announced(addresses, noise_key(1), identity(1))
            .await;

        let agent = agents.resolve_requesting_agent(addresses).await.unwrap();
        assert!(!agent.announced);
        assert_eq!(agent.noise_key, noise_key(2));
    }

    #[tokio::test]
    async fn on_chain_pairs_are_recovered_via_the_noise_key() {
        let first = addresses(1789);
        let second = addresses(1790);
        let (first_key, second_key) = (noise_key(1), noise_key(2));
        let (first_identity, second_identity) = (identity(1), identity(2));

        let restored = KnownAgents::try_from(vec![
            authorisation(second.v6, second_key, Some(second_identity)),
            authorisation(first.v4, first_key, Some(first_identity)),
            authorisation(second.v4, second_key, Some(second_identity)),
            authorisation(first.v6, first_key, Some(first_identity)),
        ])
        .unwrap();

        let restored_first = restored.resolve_requesting_agent(first).await.unwrap();
        assert_eq!(restored_first.mix_v6, first.v6);
        assert_eq!(restored_first.noise_key, first_key);
        assert_eq!(restored_first.ed25519_identity, first_identity);
        assert!(restored_first.announced);

        let restored_second = restored.resolve_requesting_agent(second).await.unwrap();
        assert_eq!(restored_second.mix_v6, second.v6);
        assert_eq!(restored_second.noise_key, second_key);
        assert_eq!(restored_second.ed25519_identity, second_identity);
        assert!(restored_second.announced);
    }

    // on-chain records that don't form an ipv4/ipv6 pair (authorisations predating the ipv6
    // announcement, or an address the agent no longer announces) are dropped, so the agent
    // re-announces itself and pays for one extra contract tx
    #[tokio::test]
    async fn unpaired_on_chain_records_are_dropped() {
        let agent = addresses(1789);
        let key = noise_key(1);
        let id = Some(identity(1));

        let v4_only = KnownAgents::try_from(vec![authorisation(agent.v4, key, id)]).unwrap();
        assert!(v4_only.resolve_requesting_agent(agent).await.is_none());

        let stale_v6 = KnownAgents::try_from(vec![
            authorisation(agent.v4, key, id),
            authorisation(agent.v6, key, id),
            authorisation("[bbbb::1]:1789".parse().unwrap(), key, id),
        ])
        .unwrap();
        assert!(stale_v6.resolve_requesting_agent(agent).await.is_none());
    }

    // a pair that doesn't carry one identity across both entries is dropped rather than restored
    // without one: an entry predating the field would otherwise be cached as announced, and the
    // orchestrator would hand work to an agent whose on-chain entry can't grant a gateway session.
    // the next announcement rebuilds it complete, and the contract's upsert overwrites in place
    #[tokio::test]
    async fn an_on_chain_pair_without_one_identity_is_dropped() {
        let agent = addresses(1789);
        let key = noise_key(1);

        for (v4_identity, v6_identity) in [
            // authorised before the identity field existed
            (None, None),
            // only one of the two addresses has been re-authorised since
            (Some(identity(1)), None),
            (None, Some(identity(1))),
            // a half-written pair: the two entries disagree on who the agent is
            (Some(identity(1)), Some(identity(2))),
        ] {
            let restored = KnownAgents::try_from(vec![
                authorisation(agent.v4, key, v4_identity),
                authorisation(agent.v6, key, v6_identity),
            ])
            .unwrap();
            assert!(restored.resolve_requesting_agent(agent).await.is_none());
        }
    }
}
