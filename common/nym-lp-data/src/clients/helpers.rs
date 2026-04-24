// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::clients::traits::{Obfuscation, Reliability, RoutingSecurity};
use crate::clients::{InputOptions, PipelinePayload};

/// Marker trait for a no-op [`Reliability`] implementation.
///
/// Implement this for your pipeline type to get a [`Reliability`] impl that
/// passes the payload through unchanged with zero byte overhead.
pub trait NoOpReliability {}

impl<T, Ts, Opts, NdId> Reliability<Ts, Opts, NdId> for T
where
    T: NoOpReliability,
    Opts: InputOptions<NdId>,
{
    const OVERHEAD_SIZE: usize = 0;
    fn reliable_encode(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        _: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>> {
        input.map(|payload| vec![payload]).unwrap_or_default()
    }
}

/// Marker trait for a no-op [`RoutingSecurity`] implementation.
///
/// Implement this for your pipeline type to get a [`RoutingSecurity`] impl that
/// passes the payload through unchanged with zero byte overhead and `nb_frames() == 1`.
pub trait NoOpRoutingSecurity {}

impl<T, Ts, Opts, NdId> RoutingSecurity<Ts, Opts, NdId> for T
where
    T: NoOpRoutingSecurity,
    Opts: InputOptions<NdId>,
{
    const OVERHEAD_SIZE: usize = 0;

    fn nb_frames(&self) -> usize {
        1
    }

    fn encrypt(
        &mut self,
        input: PipelinePayload<Ts, Opts, NdId>,
    ) -> PipelinePayload<Ts, Opts, NdId> {
        input
    }
}

/// Marker trait for a no-op [`Obfuscation`] implementation.
///
/// Implement this for your pipeline type to get an [`Obfuscation`] impl that
/// passes the input through unchanged with no cover traffic, delay, or
/// buffering.
pub trait NoOpObfuscation {}

impl<T, Ts, Opts, NdId> Obfuscation<Ts, Opts, NdId> for T
where
    T: NoOpObfuscation,
    Opts: InputOptions<NdId>,
{
    fn obfuscate(
        &mut self,
        input: Option<PipelinePayload<Ts, Opts, NdId>>,
        _: Ts,
    ) -> Vec<PipelinePayload<Ts, Opts, NdId>> {
        input.map(|payload| vec![payload]).unwrap_or_default()
    }
}
