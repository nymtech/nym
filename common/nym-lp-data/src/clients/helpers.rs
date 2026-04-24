// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::TimedPayload;
use crate::clients::traits::{Obfuscation, Reliability, RoutingSecurity};

/// Marker trait for a no-op [`Reliability`] implementation.
///
/// Implement this for your pipeline type to get a [`Reliability`] impl that
/// passes the payload through unchanged with zero byte overhead.
pub trait NoOpReliability {}

impl<T, Ts> Reliability<Ts> for T
where
    T: NoOpReliability,
{
    const OVERHEAD_SIZE: usize = 0;
    fn reliable_encode(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

/// Marker trait for a no-op [`RoutingSecurity`] implementation.
///
/// Implement this for your pipeline type to get a [`RoutingSecurity`] impl that
/// passes the payload through unchanged with zero byte overhead and `nb_frames() == 1`.
pub trait NoOpRoutingSecurity {}

impl<T, Ts> RoutingSecurity<Ts> for T
where
    T: NoOpRoutingSecurity,
{
    const OVERHEAD_SIZE: usize = 0;

    fn nb_frames(&self) -> usize {
        1
    }

    fn encrypt(&self, input: TimedPayload<Ts>) -> TimedPayload<Ts> {
        input
    }
}

/// Marker trait for a no-op [`Obfuscation`] implementation.
///
/// Implement this for your pipeline type to get an [`Obfuscation`] impl that
/// passes the input through unchanged with no cover traffic, delay, or
/// buffering.
pub trait NoOpObfuscation {}

impl<T, Ts> Obfuscation<Ts> for T
where
    T: NoOpObfuscation,
{
    fn obfuscate(&mut self, input: Option<TimedPayload<Ts>>, _: Ts) -> Vec<TimedPayload<Ts>> {
        input.map(|payload| vec![payload]).unwrap_or_default()
    }
    fn buffer_size(&self) -> usize {
        0
    }
}
