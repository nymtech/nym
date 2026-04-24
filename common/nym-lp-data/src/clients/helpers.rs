// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::TimedPayload;
use crate::clients::traits::{Obfuscation, Reliability, RoutingSecurity};

/// No-op [`Reliability`] implementation.
///
/// Passes the payload through unchanged with zero byte overhead.
/// Use this when reliability encoding (e.g. KCP) is not required.
pub struct NoOpReliability;

impl<Ts> Reliability<Ts> for NoOpReliability {
    const OVERHEAD_SIZE: usize = 0;
    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

/// No-op [`RoutingSecurity`] implementation.
///
/// Passes the payload through unchanged with zero byte overhead and
/// `nb_frames() == 1`.  Use this when Sphinx (or other onion) encryption is
/// not required.
pub struct NoOpRoutingSecurity;

impl<Ts> RoutingSecurity<Ts> for NoOpRoutingSecurity {
    const OVERHEAD_SIZE: usize = 0;

    fn nb_frames(&self) -> usize {
        1
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

/// No-op [`Obfuscation`] implementation.
///
/// Returns the input payload as a single-element vector immediately, with no
/// cover traffic, delay, or buffering.  Use this when traffic-shape obfuscation
/// is not required.
pub struct NoOpObfusctation;

impl<Ts> Obfuscation<Ts> for NoOpObfusctation {
    fn obfuscate(&mut self, input: TimedPayload<Ts>, _: Ts) -> Vec<TimedPayload<Ts>> {
        vec![input]
    }
    fn buffer_size(&self) -> usize {
        0
    }
}
