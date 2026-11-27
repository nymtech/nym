// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::helpers::bandwidth_controller_without_tickets;
use crate::cli::five_hop_mixnet::FiveHopMixnetArgs;
use anyhow::{Context, bail};
use nym_bin_common::output_format::OutputFormat;
use nym_network_monitor_route_client::config::RouteNodeSelection;
use nym_network_monitor_route_client::session::{SessionGateway, SessionOutcome, run_sessions};
use nym_network_monitor_route_records::NodeId;
use nym_network_monitor_route_records::batch::Batch;
use nym_network_monitor_route_records::scoring::{FailSequenceRule, ScoringStrategy};
use nym_network_monitor_route_records::test_message::{SessionTestMessages, merge_in_send_order};
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::NymApiClientExt;
use rand::seq::IndexedRandom;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;
use tracing::info;
use url::Url;

/// Arguments for the `test-five-hop-liveness` subcommand.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    five_hop_mixnet: FiveHopMixnetArgs,

    /// Node id of a gateway to run a session through; repeat it for each gateway, or leave it out to draw them at random.
    #[arg(long = "gateway", value_delimiter = ',', env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_GATEWAYS_ARG)]
    gateways: Vec<NodeId>,

    /// How many gateways to draw at random when none is given.
    #[arg(long, default_value = "2", conflicts_with = "gateways", env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_RANDOM_GATEWAYS_ARG)]
    random_gateways: NonZeroUsize,

    /// Lowest nym-api performance, in percent, of a gateway drawn at random.
    #[arg(long, default_value_t = 99, value_parser = clap::value_parser!(u8).range(0..=100), conflicts_with = "gateways", env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_RANDOM_GATEWAY_MIN_PERFORMANCE_ARG)]
    random_gateway_min_performance: u8,

    /// Which mixnodes the sessions route through; anything but the active set is for experiments only.
    #[arg(long, value_enum, default_value_t = NodeSelection::ActiveSet, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_NODE_SELECTION_ARG)]
    node_selection: NodeSelection,

    /// Margin, in [0, 1], by which a session's loss rate may exceed its run's best before it is discarded.
    #[arg(long, default_value_t = 0.05, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_SESSION_LOSS_MARGIN_ARG)]
    five_hop_liveness_session_loss_margin: f64,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl Args {
    /// The named gateways with their identity keys, or a random draw among those reaching the minimum performance, all as nym-api lists them.
    async fn five_hop_mixnet_gateways(
        &self,
        nym_api_url: &Url,
    ) -> anyhow::Result<Vec<SessionGateway>> {
        let nym_api = nym_http_api_client::Client::builder(nym_api_url.clone())?
            .no_hickory_dns()
            .with_user_agent(format!(
                "nym-network-monitor-agent/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        let listed = nym_api
            .get_all_basic_entry_assigned_nodes_with_metadata()
            .await
            .context("could not read the gateways from nym-api")?
            .nodes;

        if self.gateways.is_empty() {
            let min_performance = self.random_gateway_min_performance;
            let requested = self.random_gateways.get();
            let qualifying: Vec<_> = listed
                .iter()
                .filter(|gateway| gateway.performance.round_to_integer() >= min_performance)
                .collect();
            if qualifying.len() < requested {
                bail!(
                    "only {} of nym-api's gateways reach {min_performance}% performance, {requested} were asked for",
                    qualifying.len()
                )
            }

            let drawn: Vec<SessionGateway> = qualifying
                .sample(&mut rand::rng(), requested)
                .map(|gateway| SessionGateway {
                    node_id: gateway.node_id,
                    identity_key: gateway.ed25519_identity_pubkey,
                })
                .collect();
            info!(
                "drew gateways {:?} at random",
                drawn
                    .iter()
                    .map(|gateway| gateway.node_id)
                    .collect::<Vec<_>>()
            );
            return Ok(drawn);
        }

        self.gateways
            .iter()
            .map(|&node_id| {
                let gateway = listed
                    .iter()
                    .find(|gateway| gateway.node_id == node_id)
                    .with_context(|| format!("node {node_id} is not among nym-api's gateways"))?;
                Ok(SessionGateway {
                    node_id,
                    identity_key: gateway.ed25519_identity_pubkey,
                })
            })
            .collect()
    }
}

/// Which mixnodes a manual run routes through.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum NodeSelection {
    /// The mixnodes of the active set, as a five-hop mixnet run handed out by the orchestrator.
    ActiveSet,

    /// The active set and the standby nodes of the rewarded set.
    RewardedSet,

    /// Every node declaring the mixnode role.
    AllMixnodes,
}

impl From<NodeSelection> for RouteNodeSelection {
    fn from(selection: NodeSelection) -> Self {
        match selection {
            NodeSelection::ActiveSet => RouteNodeSelection::ActiveSet,
            NodeSelection::RewardedSet => RouteNodeSelection::RewardedSet,
            NodeSelection::AllMixnodes => RouteNodeSelection::AllMixnodes,
        }
    }
}

/// Runs one session through each gateway and prints the sessions and the five-hop liveness they give.
pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let session_loss_margin = args.five_hop_liveness_session_loss_margin;
    if !(0.0..=1.0).contains(&session_loss_margin) {
        bail!("five-hop liveness session loss margin ({session_loss_margin}) must be within [0, 1]")
    }

    let config = args
        .five_hop_mixnet
        .build_config(args.node_selection.into())?;
    let gateways = args
        .five_hop_mixnet_gateways(&config.session.nym_api_url)
        .await?;

    let outcomes = run_sessions(
        &config.session,
        gateways,
        &bandwidth_controller_without_tickets(),
        // a manual run IS the whole process, so its run is the root
        &ShutdownToken::new(),
        config.timeout,
    )
    .await?;

    args.output
        .to_stdout(&FiveHopMixnetRunReport::new(outcomes, session_loss_margin));
    Ok(())
}

/// What a manual five-hop mixnet run found.
#[derive(Serialize)]
struct FiveHopMixnetRunReport {
    /// The epoch of the active set the sessions used, if any session finished.
    mixnet_epoch: Option<u32>,

    /// Each session, in the order its gateway was given.
    sessions: Vec<FiveHopMixnetSessionReport>,

    /// Each scored node's five-hop liveness.
    five_hop_liveness: BTreeMap<NodeId, f64>,
}

impl FiveHopMixnetRunReport {
    /// Judges each finished session against the run as the orchestrator does, and scores the kept ones.
    fn new(outcomes: Vec<SessionOutcome>, session_loss_margin: f64) -> Self {
        // every session is judged against all of them, so the loss rates come first
        let run_loss_rates: Vec<f64> = outcomes
            .iter()
            .filter_map(|outcome| outcome.batch.as_ref().ok())
            .filter_map(|batch| SessionTestMessages::from_batch(batch).loss_rate())
            .collect();

        let mut mixnet_epoch = None;
        let mut sessions = Vec::new();
        let mut kept = Vec::new();
        for outcome in outcomes {
            let gateway_node_id = outcome.gateway.node_id;
            let session_outcome = match outcome.batch {
                Ok(batch) => {
                    mixnet_epoch = Some(batch.header.mixnet_epoch);
                    let test_messages = SessionTestMessages::from_batch(&batch);
                    let summary = FiveHopMixnetSessionSummary::new(&batch, &test_messages);
                    if test_messages.is_discarded(&run_loss_rates, session_loss_margin) {
                        FiveHopMixnetSessionVerdict::Discarded(summary)
                    } else {
                        kept.push(test_messages);
                        FiveHopMixnetSessionVerdict::Kept(summary)
                    }
                }
                Err(err) => FiveHopMixnetSessionVerdict::Error(format!("{err:#}")),
            };
            sessions.push(FiveHopMixnetSessionReport {
                gateway_node_id,
                outcome: session_outcome,
            });
        }

        // no one-hop scores here: the fail-sequence rule does not read them
        let five_hop_liveness = FailSequenceRule.score(&merge_in_send_order(kept), &HashMap::new());

        FiveHopMixnetRunReport {
            mixnet_epoch,
            sessions,
            five_hop_liveness,
        }
    }
}

impl Display for FiveHopMixnetRunReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.mixnet_epoch {
            Some(mixnet_epoch) => write!(f, "mixnet epoch {mixnet_epoch}")?,
            None => write!(f, "no session finished")?,
        }
        for session in &self.sessions {
            write!(f, "\n{session}")?;
        }
        write!(
            f,
            "\nfive-hop liveness of {} nodes:",
            self.five_hop_liveness.len()
        )?;
        for (node_id, five_hop_liveness) in &self.five_hop_liveness {
            write!(f, "\n  node {node_id}: {five_hop_liveness:.3}")?;
        }
        Ok(())
    }
}

/// What became of the session through one gateway.
#[derive(Serialize)]
struct FiveHopMixnetSessionReport {
    gateway_node_id: NodeId,

    outcome: FiveHopMixnetSessionVerdict,
}

impl Display for FiveHopMixnetSessionReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let gateway_node_id = self.gateway_node_id;
        match &self.outcome {
            FiveHopMixnetSessionVerdict::Kept(summary) => {
                write!(
                    f,
                    "session through gateway {gateway_node_id}: kept, {summary}"
                )
            }
            FiveHopMixnetSessionVerdict::Discarded(summary) => {
                write!(
                    f,
                    "session through gateway {gateway_node_id}: discarded, {summary}"
                )
            }
            FiveHopMixnetSessionVerdict::Error(error) => {
                write!(
                    f,
                    "session through gateway {gateway_node_id}: failed: {error}"
                )
            }
        }
    }
}

/// A session's verdict with what its records hold, or its error.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum FiveHopMixnetSessionVerdict {
    Kept(FiveHopMixnetSessionSummary),

    Discarded(FiveHopMixnetSessionSummary),

    Error(String),
}

/// A finished session's records: how many, how many payloads and acks came back, and the share of test messages lost.
#[derive(Serialize)]
struct FiveHopMixnetSessionSummary {
    record_count: usize,

    payloads_back: usize,

    acks_back: usize,

    loss_rate: Option<f64>,
}

impl FiveHopMixnetSessionSummary {
    fn new(batch: &Batch, test_messages: &SessionTestMessages) -> Self {
        FiveHopMixnetSessionSummary {
            record_count: batch.records.len(),
            payloads_back: batch
                .records
                .iter()
                .filter(|record| record.payload_back)
                .count(),
            acks_back: batch
                .records
                .iter()
                .filter(|record| record.ack_back)
                .count(),
            loss_rate: test_messages.loss_rate(),
        }
    }

    /// Percentage of the session's records not among the `back` that came back; only called with records.
    fn percent_not_back(&self, back: usize) -> f64 {
        100.0 * (self.record_count - back) as f64 / self.record_count as f64
    }
}

impl Display for FiveHopMixnetSessionSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} records", self.record_count)?;
        let Some(loss_rate) = self.loss_rate else {
            return Ok(());
        };
        write!(
            f,
            ", {:.2}% of payloads and {:.2}% of acks not back, {:.2}% of test messages lost",
            self.percent_not_back(self.payloads_back),
            self.percent_not_back(self.acks_back),
            loss_rate * 100.0
        )
    }
}
