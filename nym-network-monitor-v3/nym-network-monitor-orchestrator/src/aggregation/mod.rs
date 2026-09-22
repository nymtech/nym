// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Turning stored test runs into the per-node, per-epoch, per-kind figures the orchestrator serves.
//!
//! Every rule about what a number MEANS lives here rather than in the storage layer, which only
//! keeps the rows, or in the HTTP layer, which only hands them out.

use crate::storage::models::{
    ExercisedInterface, NodeSamples, TestKind, TestPairing, TestRunMeasurement, TestedRole,
};
use nym_network_monitor_orchestrator_requests::models::InterfaceMeasurement;

pub(crate) mod materialiser;

/// What one `(node, kind)` amounted to over one window, and the evidence behind it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct WindowAggregate {
    /// Mean of the scores of the runs that came back.
    pub(crate) score: f64,

    /// How many runs that mean was taken over. Never zero: a window that returned nothing produces
    /// no aggregate at all rather than one resting on no evidence.
    pub(crate) samples: usize,
}

/// The interfaces a run of this pairing is expected to have exercised, which is the set its score
/// averages over.
///
/// Averaging over the EXPECTED set rather than over whatever came back is what stops a phase that
/// produced nothing from shrinking the denominator: a gateway whose delivery never ran must not tie
/// with one that passed both.
fn expected_interfaces(pairing: TestPairing) -> &'static [ExercisedInterface] {
    match (pairing.test_kind, pairing.tested_role) {
        // the stress probe drives the forwarding path and nothing else, whatever other roles the
        // node under test happens to serve
        (TestKind::Stress, _) => &[ExercisedInterface::MixForwarding],
        (TestKind::Liveness, TestedRole::Mixnode) => &[ExercisedInterface::MixForwarding],
        (TestKind::Liveness, TestedRole::Gateway) => &[
            ExercisedInterface::ClientIngest,
            ExercisedInterface::ClientDelivery,
        ],
    }
}

/// Delivery ratio against one interface, zero if the run produced no measurement for it.
///
/// A measurement that saw duplicates is discarded whole rather than scored, because an honest node
/// never replays a packet and the ratio alone cannot tell the two apart: a node that forwards one
/// packet and echoes it nine more times counts ten received against ten sent and would otherwise
/// score a perfect 1.0 for having delivered a tenth of the traffic.
fn interface_performance(
    measurements: &[TestRunMeasurement],
    interface: ExercisedInterface,
) -> f64 {
    match measurements
        .iter()
        .find(|measurement| measurement.interface == interface)
    {
        // the ratio (and its clamp) is defined once, on the API-level measurement
        Some(measurement) if !measurement.received_duplicates => {
            InterfaceMeasurement::from(measurement).received_ratio()
        }
        _ => 0.0,
    }
}

/// What a run measured, as one number in `[0.0, 1.0]`: the mean of its delivery ratios over the
/// interfaces its pairing was expected to exercise.
///
/// Defined once because two consumers have to agree on it. This is the figure submitted to the
/// nym-api AND the score a sample records, and the whole point of running both systems side by side
/// is that a discrepancy between their numbers is attributable to something other than the
/// definition.
///
/// A run carrying a run-level error is scored the same way rather than forced to zero. It does not
/// need to be: an interface that sent nothing already rates zero, so a run that failed before
/// measuring anything scores zero of its own accord, and the only runs the distinction would touch
/// are those that DID measure something before aborting - a probe that exceeded its deadline
/// part-way through its load test, say. For those the measured ratio is the truer statement about
/// the node, and forcing it to zero would put this figure and the nym-api's out of step for no gain.
pub(crate) fn run_performance(pairing: TestPairing, measurements: &[TestRunMeasurement]) -> f64 {
    let expected = expected_interfaces(pairing);
    let total: f64 = expected
        .iter()
        .map(|&interface| interface_performance(measurements, interface))
        .sum();

    total / expected.len() as f64
}

/// What a window's samples amount to for one `(node, kind)`, or `None` when none came back.
///
/// The mean of the per-run scores, deliberately NOT a delivery ratio pooled from the packet counts
/// underneath them. Three reasons, the last decisive: a run's own score is already an average over
/// its kind's fixed measurement set, so this is consistent with the layer below; the metric is
/// availability over time, so one observation should weigh the same whether it happened to send 50
/// packets or 37; and while both systems run, this and the nym-api compute from the SAME runs, so
/// pooling here while the nym-api averages would make the two disagree by construction and leave no
/// discrepancy attributable to a bug rather than to the definition.
///
/// Every role and every announced address that kind exercised for the node falls into the one
/// average, because a sample records neither. The contract is keyed per node, so the collapse has to
/// happen somewhere, and a node whose ipv6 address is broken carrying genuine zeros into its value
/// is the intended reading rather than a defect - though it does mean a half-scoring node needs the
/// per-run surface to diagnose.
///
/// `None` rather than zero when nothing came back, because a node that was not measured must never
/// be publishable as one measured at zero. Runs that returned an error are already in here, having
/// been scored like any other when they arrived; it is the assignments still waiting on a result
/// that are absent, and they are not a statement about the node.
pub(crate) fn aggregate_window(samples: &NodeSamples) -> Option<WindowAggregate> {
    if samples.scores.is_empty() {
        return None;
    }

    let total: f64 = samples.scores.iter().sum();
    Some(WindowAggregate {
        score: total / samples.scores.len() as f64,
        samples: samples.scores.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn returned(scores: Vec<f64>) -> NodeSamples {
        NodeSamples {
            scores,
            unreturned: 0,
        }
    }

    // Decision 2, stated as a number. The two runs behind these scores sent 50 and 3 packets, and
    // pooling those counts would give 50/53, or about 0.94 - a node that failed a short run would
    // read as almost perfect. Each observation weighs the same instead, because the metric is
    // availability over time rather than a delivery probability.
    //
    // Pooling is in fact unreachable from here: a sample records a score and not the counts under
    // it, so this pins the meaning rather than guarding the arithmetic.
    #[test]
    fn each_run_weighs_the_same_regardless_of_how_many_packets_it_sent() {
        let aggregate = aggregate_window(&returned(vec![1.0, 0.0])).expect("nothing aggregated");

        assert_eq!(aggregate.score, 0.5);
        assert_eq!(aggregate.samples, 2);
    }

    // a node that was not measured must never be publishable as one measured at zero, and the
    // contract's interface cannot carry the difference, so it has to be made here
    #[test]
    fn a_window_that_returned_nothing_produces_no_aggregate() {
        assert!(aggregate_window(&returned(vec![])).is_none());

        // and assignments alone are not evidence: they say the monitor tried, not what the node did
        let assigned_but_silent = NodeSamples {
            scores: vec![],
            unreturned: 4,
        };
        assert!(aggregate_window(&assigned_but_silent).is_none());
    }
}
