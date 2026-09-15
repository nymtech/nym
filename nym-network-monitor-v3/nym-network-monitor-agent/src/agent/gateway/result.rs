// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! What a gateway liveness run measures.
//!
//! Deliberately holds no result type of its own. A gateway run is an ordinary
//! [`TestRunResult`](crate::agent::result::TestRunResult) whose expected set names two interfaces
//! instead of one, so the run-level frame, the per-interface
//! [`PacketDelivery`](crate::agent::result::PacketDelivery) and the projection onto the wire are all
//! shared with the mixnode path rather than reimplemented here. What belongs here is only the fact of
//! WHICH interfaces a gateway probe is defined to exercise.

use nym_network_monitor_orchestrator_requests::models::ExercisedInterface;

/// The interfaces a gateway liveness run is required to carry, and the order they are reported in.
///
/// Both, always. The kind fixes the score's denominator at this set's length, so a phase that
/// produced nothing is submitted as a zero: were it omitted instead, the average would be taken over
/// one measurement and a gateway whose delivery never ran would tie with one that passed both.
pub(crate) const GATEWAY_EXERCISED_INTERFACES: &[ExercisedInterface] = &[
    ExercisedInterface::ClientIngest,
    ExercisedInterface::ClientDelivery,
];
