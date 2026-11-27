// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_monitor_orchestrator_requests::models::{
    FiveHopMixnetAssignment, FiveHopMixnetResult, RouteSessionOutcome, RouteSessionSubmission,
};
use nym_network_monitor_route_client::session::SessionOutcome;
use tracing::{info, warn};

/// The run's submission: each session's batch or error, or the failed read's error for every gateway.
pub(crate) fn five_hop_mixnet_result(
    assignment: &FiveHopMixnetAssignment,
    outcomes: anyhow::Result<Vec<SessionOutcome>>,
) -> FiveHopMixnetResult {
    let sessions = match outcomes {
        Ok(outcomes) => outcomes.into_iter().map(session_submission).collect(),
        Err(err) => {
            warn!("no five-hop mixnet session started: {err:#}");
            assignment
                .gateways
                .iter()
                .map(|gateway| RouteSessionSubmission {
                    gateway_node_id: gateway.node_id,
                    outcome: RouteSessionOutcome::Error(format!("{err:#}")),
                })
                .collect()
        }
    };

    FiveHopMixnetResult {
        run_id: assignment.id,
        sessions,
    }
}

/// One session's entry: its encoded batch, or its error.
fn session_submission(outcome: SessionOutcome) -> RouteSessionSubmission {
    let gateway_node_id = outcome.gateway.node_id;
    let outcome = match outcome.batch {
        Ok(batch) => {
            info!(
                gateway_node_id,
                records = batch.records.len(),
                "five-hop mixnet session finished"
            );
            RouteSessionOutcome::Batch(batch.encode())
        }
        Err(err) => {
            warn!(gateway_node_id, "five-hop mixnet session failed: {err:#}");
            RouteSessionOutcome::Error(format!("{err:#}"))
        }
    };

    RouteSessionSubmission {
        gateway_node_id,
        outcome,
    }
}
