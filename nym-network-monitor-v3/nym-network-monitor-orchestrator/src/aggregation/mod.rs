// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Turning stored test runs into the per-node, per-epoch, per-kind figures the orchestrator serves.
//!
//! Every rule about what a number MEANS lives here rather than in the storage layer, which only
//! keeps the rows, or in the HTTP layer, which only hands them out.

use crate::storage::models::{
    ExercisedInterface, TestKind, TestPairing, TestRunMeasurement, TestedRole,
};
use nym_network_monitor_orchestrator_requests::models::InterfaceMeasurement;

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
