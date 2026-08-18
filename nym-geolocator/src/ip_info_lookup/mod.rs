// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::helpers::ip_info_to_location;
use anyhow::bail;
use ipinfo::{BatchReqOpts, IpDetails, IpError, IpErrorKind, IpInfoConfig};
use nym_geolocation_contract_common::payload::Location;
use nym_validator_client::client::NodeId;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tokio::time::timeout;
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

/// How long an http handler waits for the provider client before giving up on it.
///
/// The client is shared with the sweep and can only be used by one caller at a time (see
/// [`IpInfoLookup`]), so a request arriving mid-sweep waits for the chunk in flight. Short on
/// purpose: a caller is better told to come back than left holding a connection open, and the
/// sweep releases the client between chunks so an ordinary wait is far below this.
const HTTP_LOOKUP_LOCK_TIMEOUT: Duration = Duration::from_secs(1);

/// Why a single-node lookup could not be served.
pub(crate) enum LookupError {
    /// The provider client was busy with another lookup for longer than the caller could wait.
    /// Nothing was asked of the provider, so retrying is free.
    Busy,

    /// The lookup itself failed.
    Failed(anyhow::Error),
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
}

/// Shared access to the lookup provider.
///
/// Every entry point of `ipinfo::IpInfo` takes `&mut self`, since the client owns an internal
/// cache, so provider access is serialised by construction rather than by choice of lock. The
/// sweep therefore splits its work into chunks and releases the client between them, and the http
/// handlers - which share this client with the sweep - wait only [`HTTP_LOOKUP_LOCK_TIMEOUT`] for
/// it before shedding the request.
#[derive(Clone)]
pub(crate) struct IpInfoLookup {
    max_addresses_per_lookup: usize,
    inner: Arc<Mutex<IpInfoLookupInner>>,
}

impl IpInfoLookup {
    pub(crate) fn new(config: Config, token: String) -> anyhow::Result<Self> {
        Ok(IpInfoLookup {
            max_addresses_per_lookup: config.max_addresses_per_lookup.max(1),
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

    /// Locate a single node. The counterpart of [`Self::lookup_node_locations`] for the http
    /// handlers, which act on one node at a time.
    ///
    /// Unlike the sweep this refuses to queue: the caller is an http request, so waiting out a
    /// chunk of somebody else's sweep is worse for it than being told to ask again.
    pub(crate) async fn lookup_node_location(
        &self,
        ips: Vec<IpAddr>,
    ) -> Result<Option<Location>, LookupError> {
        if ips.is_empty() {
            return Ok(None);
        }

        // only the wait for the client is bounded here, not the lookup itself - that already has
        // the provider's own timeout on it
        let mut guard = timeout(HTTP_LOOKUP_LOCK_TIMEOUT, self.inner.lock())
            .await
            .map_err(|_| LookupError::Busy)?;

        let results = guard
            .batch_lookup(&ips)
            .await
            .map_err(LookupError::Failed)?;
        Ok(Some(
            reconcile_node_responses(results).map_err(LookupError::Failed)?,
        ))
    }

    /// Locate every node, over as few provider requests as the chunk size allows.
    ///
    /// Batched rather than one request per node: the provider takes many addresses at once, so
    /// measuring a sweep's worth of nodes individually turns a handful of round trips into
    /// hundreds, each with its own latency and its own chance of being rate limited partway
    /// through the cycle. Chunked rather than sent as one request because the provider applies one
    /// flat timeout however large a batch is, so a whole sweep in a single request is one request
    /// that must fit a budget sized for a much smaller one - and if it does not, the sweep submits
    /// nothing at all. Chunking also bounds how long the sweep holds the provider client, which
    /// the http handlers share.
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

        let mut located = HashMap::new();
        let mut unresolved: HashSet<IpAddr> = HashSet::new();
        for chunk in addresses.chunks(self.max_addresses_per_lookup) {
            match self.batch_lookup(chunk).await {
                Ok(responses) => located.extend(responses),
                Err(err) => {
                    // the chunks around this one are separate requests and are still worth making,
                    // so a failure costs the nodes in it a cycle rather than all of them
                    warn!("failed to look up {} address(es): {err}", chunk.len());
                    unresolved.extend(chunk);
                }
            }
        }

        let mut locations = Vec::with_capacity(nodes.len());
        for (node_id, ips) in nodes {
            // a node is measured against every address it announced or not at all: `reconcile`
            // picks the first address it can place, so submitting for a node whose other addresses
            // were never asked about would report a location chosen by which chunk happened to
            // work. It stays due and the next sweep asks again
            if ips.iter().any(|ip| unresolved.contains(ip)) {
                debug!(
                    "node {node_id} had addresses in a failed lookup - leaving it for the next sweep"
                );
                continue;
            }

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
