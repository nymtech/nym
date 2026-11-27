// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_monitor_orchestrator_requests::models::{
    FiveHopMixnetAssignment, FiveHopMixnetResult, FiveHopMixnetSessionOutcome,
    FiveHopMixnetSessionSubmission,
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
                .map(|gateway| FiveHopMixnetSessionSubmission {
                    gateway_node_id: gateway.node_id,
                    outcome: FiveHopMixnetSessionOutcome::Error(format!("{err:#}")),
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
fn session_submission(outcome: SessionOutcome) -> FiveHopMixnetSessionSubmission {
    let gateway_node_id = outcome.gateway.node_id;
    let outcome = match outcome.batch {
        Ok(batch) => {
            info!(
                gateway_node_id,
                records = batch.records.len(),
                "five-hop mixnet session finished"
            );
            FiveHopMixnetSessionOutcome::Batch(batch.encode())
        }
        Err(err) => {
            warn!(gateway_node_id, "five-hop mixnet session failed: {err:#}");
            FiveHopMixnetSessionOutcome::Error(format!("{err:#}"))
        }
    };

    FiveHopMixnetSessionSubmission {
        gateway_node_id,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use nym_crypto::asymmetric::ed25519;
    use nym_network_monitor_orchestrator_requests::models::FiveHopMixnetGateway;
    use nym_test_utils::helpers::deterministic_rng;

    // no session starts, yet the orchestrator must hear of every gateway it gave the run
    #[test]
    fn a_run_whose_network_read_failed_is_submitted_with_an_error_for_each_gateway() {
        let mut rng = deterministic_rng();
        let assignment = FiveHopMixnetAssignment {
            id: 7,
            gateways: [12, 34]
                .map(|node_id| FiveHopMixnetGateway {
                    node_id,
                    identity_key: *ed25519::KeyPair::new(&mut rng).public_key(),
                })
                .to_vec(),
        };
        let read_failure =
            Err(anyhow!("connection refused").context("could not read the network from nym-api"));

        let result = five_hop_mixnet_result(&assignment, read_failure);

        assert_eq!(result.run_id, 7);
        assert_eq!(result.sessions.len(), 2, "{result:?}");
        for (session, gateway) in result.sessions.iter().zip(&assignment.gateways) {
            assert_eq!(session.gateway_node_id, gateway.node_id);
            assert!(
                matches!(
                    &session.outcome,
                    FiveHopMixnetSessionOutcome::Error(error)
                        if error == "could not read the network from nym-api: connection refused"
                ),
                "{session:?}"
            );
        }
    }
}
