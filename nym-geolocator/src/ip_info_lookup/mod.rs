// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::helpers::ip_info_to_location;
use anyhow::bail;
use ipinfo::{BatchReqOpts, IpDetails, IpError, IpErrorKind, IpInfoConfig};
use nym_geolocation_contract_common::payload::Location;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

/// Report the one provider failure that is neither transient nor confined to a single address.
///
/// Everything else surfaces as a node left unmeasured this cycle and retried on the next, which is
/// ordinary. Quota exhaustion looks identical from the outside and is not: no location anywhere on
/// the network refreshes until it resets, so `checked_at` quietly stops advancing for every node
/// at once, which is precisely the freshness signal consumers read.
fn report_quota_exhaustion(err: &IpError) {
    if err.kind() == IpErrorKind::RateLimitExceededError {
        error!(
            "the ipinfo lookup quota is exhausted - no node locations will refresh until it resets: {err}"
        );
    }
}

struct CachedResponse {
    at: OffsetDateTime,
    response: IpDetails,
}

struct IpInfoLookupInner {
    client: ipinfo::IpInfo,
    cache_ttl: Duration,

    // cache of performed requests in case we had chain issues
    // note: this cache's TTL has to be way lower than the geodata TTL!
    // it should only exist for short-lived failures
    lookup_cache: HashMap<IpAddr, CachedResponse>,
}

impl IpInfoLookupInner {
    async fn batch_lookup(&mut self, ips: &[IpAddr]) -> anyhow::Result<HashMap<IpAddr, IpDetails>> {
        let mut cached_responses = HashMap::new();
        let mut ip_strings = Vec::new();
        for ip in ips {
            if let Some(cached) = self.lookup_cache.get(ip) {
                if cached.at + self.cache_ttl > OffsetDateTime::now_utc() {
                    cached_responses.insert(*ip, cached.response.clone());
                    continue;
                }
            }

            ip_strings.push(ip.to_string());
        }

        if ip_strings.is_empty() {
            return Ok(cached_responses);
        }

        let lookup_batch = ip_strings.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

        let response = self
            .client
            .lookup_batch(&lookup_batch, BatchReqOpts::default())
            .await
            .inspect_err(report_quota_exhaustion)?;
        let mut results = cached_responses;

        for (ip, res) in response {
            let Ok(ip) = ip.parse() else {
                error!("received malformed ip back from ipinfo ({ip} could not be parsed)");
                continue;
            };
            self.lookup_cache.insert(
                ip,
                CachedResponse {
                    at: OffsetDateTime::now_utc(),
                    response: res.clone(),
                },
            );
            results.insert(ip, res);
        }
        Ok(results)
    }

    async fn lookup_address(&mut self, ip: IpAddr) -> anyhow::Result<IpDetails> {
        if let Some(cached) = self.lookup_cache.get(&ip) {
            if cached.at + self.cache_ttl > OffsetDateTime::now_utc() {
                return Ok(cached.response.clone());
            }
        }

        let response = self
            .client
            .lookup(&ip.to_string())
            .await
            .inspect_err(report_quota_exhaustion)?;
        self.lookup_cache.insert(
            ip,
            CachedResponse {
                at: OffsetDateTime::now_utc(),
                response: response.clone(),
            },
        );
        Ok(response)
    }
}

#[derive(Clone)]
pub(crate) struct IpInfoLookup {
    inner: Arc<Mutex<IpInfoLookupInner>>,
}

impl IpInfoLookup {
    pub(crate) fn new(config: Config, token: String) -> anyhow::Result<Self> {
        Ok(IpInfoLookup {
            inner: Arc::new(Mutex::new(IpInfoLookupInner {
                client: ipinfo::IpInfo::new(IpInfoConfig {
                    token: Some(token),
                    ..Default::default()
                })?,
                cache_ttl: config.ip_info_lookup_cache_ttl,
                lookup_cache: HashMap::new(),
            })),
        })
    }

    pub(crate) async fn batch_lookup(
        &self,
        ips: &[IpAddr],
    ) -> anyhow::Result<HashMap<IpAddr, IpDetails>> {
        self.inner.lock().await.batch_lookup(ips).await
    }

    pub(crate) async fn lookup_address(&self, ip: IpAddr) -> anyhow::Result<IpDetails> {
        self.inner.lock().await.lookup_address(ip).await
    }

    /// Locate a single node. The counterpart of [`Self::lookup_node_locations`] for the http
    /// handlers, which act on one node at a time.
    pub(crate) async fn lookup_node_location(
        &self,
        ips: Vec<IpAddr>,
    ) -> anyhow::Result<Option<Location>> {
        if ips.is_empty() {
            return Ok(None);
        }

        if ips.len() == 1 {
            let location = self.lookup_address(ips[0]).await?;
            return Ok(Some(ip_info_to_location(location)?));
        }

        let results = self.batch_lookup(&ips).await?;
        Ok(Some(reconcile_node_responses(results)?))
    }

    /// Locate every node in one pass over the provider.
    ///
    /// A single batch request for the whole tick rather than one per node: the provider takes
    /// every address at once, so measuring a sweep's worth of nodes individually turns one round
    /// trip into hundreds, each with its own latency and its own chance of being rate limited
    /// partway through the cycle.
    ///
    /// A node that could not be located is absent from the result rather than reported: there is
    /// no assertion to make for it, and it stays due so the next sweep retries it.
    pub(crate) async fn lookup_node_locations(
        &self,
        nodes: Vec<(NodeId, Vec<IpAddr>)>,
    ) -> Vec<(NodeId, Location)> {
        // deduplicated because the provider bills per address, and two nodes announcing the same
        // one - unusual, but a shared host makes it possible - must not be charged twice
        let mut addresses = nodes
            .iter()
            .flat_map(|(_, ips)| ips.iter().copied())
            .collect::<Vec<_>>();
        addresses.sort_unstable();
        addresses.dedup();

        if addresses.is_empty() {
            return Vec::new();
        }

        let located = match self.batch_lookup(&addresses).await {
            Ok(located) => located,
            Err(err) => {
                // the whole request failed, so nothing is known about any of these nodes and
                // none of them may be submitted for
                warn!("failed to look up {} address(es): {err}", addresses.len());
                return Vec::new();
            }
        };

        let mut locations = Vec::with_capacity(nodes.len());
        for (node_id, ips) in nodes {
            // addresses the provider could not place are simply missing from the response
            let responses = ips
                .into_iter()
                .filter_map(|ip| located.get(&ip).map(|details| (ip, details.clone())))
                .collect::<HashMap<_, _>>();

            match reconcile_node_responses(responses) {
                Ok(location) => locations.push((node_id, location)),
                Err(err) => debug!("no usable location for node {node_id}: {err}"),
            }
        }

        locations
    }
}

fn reconcile_node_responses(responses: HashMap<IpAddr, IpDetails>) -> anyhow::Result<Location> {
    if responses.is_empty() {
        bail!("no successful node location lookup")
    }

    // for now use naive logic. sort addresses and use the first available ipv4 response
    // (`IpAddr` orders every v4 before every v6, so this falls back to ipv6 only when the node
    // announced no ipv4). An address the provider could not place is skipped rather than
    // failing the node, so an unlocatable v4 does not veto a perfectly good v6
    let responses: BTreeMap<_, _> = responses.into_iter().collect();
    for (_, response) in responses {
        match ip_info_to_location(response) {
            Ok(location) => return Ok(location),
            Err(err) => debug!("skipping an address with no usable location: {err}"),
        }
    }

    bail!("none of the addresses yielded a usable location")
}
