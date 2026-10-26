// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use crate::PipelinePayload;
use crate::clients::traits::{Obfuscation, Reliability, RoutingSecurity};

/// Marker trait for a no-op [`Reliability`] implementation.
///
/// Implement this for your pipeline type to get a [`Reliability`] impl that
/// passes the payload through unchanged with zero byte overhead.
pub trait NoOpReliability {}

impl<T, Opts> Reliability<Opts> for T
where
    T: NoOpReliability,
{
    const OVERHEAD_SIZE: usize = 0;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Opts>>,
        _: Instant,
    ) -> Vec<PipelinePayload<Opts>> {
        input.map(|payload| vec![payload]).unwrap_or_default()
    }
}

/// Marker trait for a no-op [`RoutingSecurity`] implementation.
///
/// Implement this for your pipeline type to get a [`RoutingSecurity`] impl that
/// passes the payload through unchanged with zero byte overhead and `nb_frames() == 1`.
pub trait NoOpRoutingSecurity {}

impl<T, Opts> RoutingSecurity<Opts> for T
where
    T: NoOpRoutingSecurity,
{
    const OVERHEAD_SIZE: usize = 0;

    fn nb_frames(&self) -> usize {
        1
    }

    fn encrypt(&mut self, input: PipelinePayload<Opts>) -> PipelinePayload<Opts> {
        input
    }
}

/// Marker trait for a no-op [`Obfuscation`] implementation.
///
/// Implement this for your pipeline type to get an [`Obfuscation`] impl that
/// passes the input through unchanged with no cover traffic, delay, or
/// buffering.
pub trait NoOpObfuscation {}

impl<T, Opts> Obfuscation<Opts> for T
where
    T: NoOpObfuscation,
{
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Opts>>,
        _: Instant,
    ) -> Vec<PipelinePayload<Opts>> {
        input.map(|payload| vec![payload]).unwrap_or_default()
    }
}
