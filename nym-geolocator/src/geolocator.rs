// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::ip_info_lookup::IpInfoLookup;
use crate::node_scraper::NodeScraper;
use crate::node_scraper::nodes::NodeUpdate;
use crate::nyx::client::NyxClient;
use crate::nyx::location_pusher::LocationPusher;
use crate::nyx::nodes::{BondedNymNodes, get_bonded_nodes};
use crate::nyx::state::{OnChainNodes, has_expired};
use nym_task::ShutdownToken;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::HashSet;
use std::net::IpAddr;
use time::OffsetDateTime;
use tracing::{debug, error, trace};

pub(crate) struct Geolocator {
    config: Config,

    client: NyxClient,
    location_pusher: LocationPusher,

    bonded_nym_nodes: BondedNymNodes,
    on_chain_nodes: OnChainNodes,

    scraper: NodeScraper,
    ip_info_lookup: IpInfoLookup,

    shutdown: ShutdownToken,
}

impl Geolocator {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: Config,
        client: NyxClient,
        location_pusher: LocationPusher,
        bonded_nym_nodes: BondedNymNodes,
        on_chain_nodes: OnChainNodes,
        scraper: NodeScraper,
        ip_info_lookup: IpInfoLookup,
        shutdown: ShutdownToken,
    ) -> Self {
        Geolocator {
            config,
            client,
            location_pusher,
            bonded_nym_nodes,
            on_chain_nodes,
            scraper,
            ip_info_lookup,
            shutdown,
        }
    }

    async fn handle_bonded_nodes_update_tick(&mut self) -> anyhow::Result<()> {
        let updated_view = get_bonded_nodes(&self.client).await?;
        self.bonded_nym_nodes.update(updated_view).await;
        Ok(())
    }

    async fn handle_described_nodes_update_tick(&mut self) -> anyhow::Result<()> {
        // get list of nodes that have updated their ip addresses (or just appeared for the first time)
        let node_updates = self.scraper.get_updated_nodes().await;

        let mut to_measure = Vec::new();
        for update in node_updates {
            let details = match update {
                NodeUpdate::IpChanged(details) => details,
                NodeUpdate::NewNode(details) => {
                    // check if the node has already been checked before - it might have temporarily gone down
                    if !self
                        .on_chain_nodes
                        .has_expired(details.node_id, self.config.geolocation_data_ttl)
                        .await
                    {
                        continue;
                    }
                    details
                }
            };

            to_measure.push((details.node_id, details.addresses));
        }

        self.measure_and_submit(to_measure).await
    }

    /// Look up every given node, submit what resolved, and record what was measured.
    ///
    /// Nothing is recorded unless the whole submission went through. A partial failure therefore
    /// costs this set another round of lookups on the next tick, which is the deliberate trade:
    /// the alternative is marking a node measured on the strength of a batch that never reached
    /// the chain, and that node would then wait out its full ttl before anything looked at it
    /// again.
    async fn measure_and_submit(
        &self,
        to_measure: Vec<(NodeId, Vec<IpAddr>)>,
    ) -> anyhow::Result<()> {
        // what each node was measured against, kept so the mark records those addresses rather
        // than whatever the node happens to announce by the time the submission returns
        let measured_against = to_measure.clone();

        // one provider round trip per chunk, rather than one per node
        let chain_updates = self.ip_info_lookup.lookup_node_locations(to_measure).await;
        let submitted = chain_updates
            .iter()
            .map(|(node_id, _)| *node_id)
            .collect::<HashSet<_>>();

        self.location_pusher.push_updates(chain_updates).await?;

        self.scraper
            .mark_measured(
                measured_against
                    .into_iter()
                    .filter(|(node_id, _)| submitted.contains(node_id))
                    .collect(),
            )
            .await;
        Ok(())
    }

    async fn handle_expiration_tick(&mut self) -> anyhow::Result<()> {
        // an unbonded node's entries were already deleted by the contract's unbond callback, so
        // re-measuring one here would write it straight back - the measurement path does no
        // bonding check - and nothing could ever delete it again
        let bonded = self.scraper.bonded_ids().await;
        self.on_chain_nodes.retain_bonded(&bonded).await;

        // driven by the nodes this agent can measure, not by what it has already submitted. An
        // on-chain entry can only ever be found *expired*, never *missing*, so iterating that map
        // skips every node with nothing on chain yet - which on a fresh agent is all of them, and
        // the described tick does not cover them either once `KnownNodes::build_new` has recorded
        // their addresses. The service would scrape forever and submit nothing.
        //
        // `has_expired` already answers "missing or stale" for both, so the two cases stay one
        // condition. Collected before the lookups, which need `&mut self`, and capped so a cold
        // start - and the synchronised mass expiry that otherwise follows from it, since
        // everything measured in one sweep expires in one sweep - is spread over several sweeps
        // rather than arriving at the provider at once
        let ttl = self.config.geolocation_data_ttl;
        let candidates = self.scraper.known_node_ids().await;
        let due = {
            // one read guard for the whole selection rather than a lock acquisition per node, and
            // scoped so it is released before the lookups below: those take a long time, and the
            // http handlers share this map now, so holding it across them would stall every
            // request for the length of a sweep
            let on_chain = self.on_chain_nodes.read().await;
            let now = OffsetDateTime::now_utc();

            candidates
                .into_iter()
                .filter(|node_id| bonded.contains(node_id))
                .filter(|node_id| {
                    // absent means never submitted, which is due exactly like a stale one
                    on_chain
                        .get(node_id)
                        .is_none_or(|checked_at| has_expired(*checked_at, now, ttl))
                })
                .take(self.config.max_nodes_measured_per_sweep)
                .collect::<Vec<_>>()
        };

        if due.is_empty() {
            trace!("no nodes are due for a geolocation refresh");
            return Ok(());
        }
        debug!("{} node(s) due for a geolocation refresh", due.len());

        let mut to_measure = Vec::with_capacity(due.len());
        for node_id in due {
            let ips = self.scraper.node_ips(node_id).await;
            if ips.is_empty() {
                // nothing to look up, which is not the same thing as a lookup that failed and
                // must not reach reconciliation as an empty set of responses
                debug!("node {node_id} announced no addresses - nothing to geolocate");
                continue;
            }

            to_measure.push((node_id, ips));
        }

        self.measure_and_submit(to_measure).await
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started Geolocator");

        let mut self_described_interval =
            tokio::time::interval(self.config.described_node_refresh_interval);
        self_described_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut bonded_interval = tokio::time::interval(self.config.bonded_nodes_refresh_interval);
        bonded_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut expiration_interval =
            tokio::time::interval(self.config.geolocation_expiration_polling_interval);
        expiration_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    trace!("Geolocator: Received shutdown");
                    break;
                }
                _ = bonded_interval.tick() => {
                    if let Err(err) = self.handle_bonded_nodes_update_tick().await {
                        error!("failed to update bonded nym nodes: {err}");
                    }
                }
                _ = self_described_interval.tick() => {
                    if let Err(err) = self.handle_described_nodes_update_tick().await {
                        error!("failed to update known nym-nodes locations: {err}");
                    }
                }
                _ = expiration_interval.tick() => {
                    if let Err(err) = self.handle_expiration_tick().await {
                        error!("failed to run regular expiration check: {err}");
                    }
                }
            }
        }
        debug!("Geolocator: Exiting");
    }
}
