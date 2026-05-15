// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{AddressedTimedData, TimedData};

pub mod driver;
pub mod helpers;
pub mod traits;
pub mod types;

pub trait InputOptions<NdId>: Clone {
    fn reliability(&self) -> bool;
    fn routing_security(&self) -> bool;
    fn obfuscation(&self) -> bool;

    fn next_hop(&self) -> NdId;
}

pub struct PipelineData<Ts, D, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
    pub data: TimedData<Ts, D>,
    pub options: Opts,
    _marker: std::marker::PhantomData<NdId>,
}

impl<Ts, D, Opts, NdId> PipelineData<Ts, D, Opts, NdId>
where
    Opts: InputOptions<NdId>,
{
    pub fn new(timestamp: Ts, data: D, options: Opts) -> Self {
        PipelineData {
            data: TimedData::new(timestamp, data),
            options,
            _marker: std::marker::PhantomData,
        }
    }

    /// Apply `op` to the data component, leaving the timestamp unchanged.
    /// `Nd` can be a different type to allow type transform as well
    pub fn data_transform<F, Nd>(self, op: F) -> PipelineData<Ts, Nd, Opts, NdId>
    where
        F: FnMut(D) -> Nd,
    {
        PipelineData {
            data: self.data.data_transform(op),
            options: self.options,
            _marker: self._marker,
        }
    }

    /// Apply `op` to the timestamp component, leaving the data unchanged.
    pub fn ts_transform<F>(self, op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        PipelineData {
            data: self.data.ts_transform(op),
            options: self.options,
            _marker: self._marker,
        }
    }
}

impl<Ts, D, Opts, NdId> From<PipelineData<Ts, D, Opts, NdId>> for AddressedTimedData<Ts, D, NdId>
where
    Opts: InputOptions<NdId>,
{
    fn from(value: PipelineData<Ts, D, Opts, NdId>) -> Self {
        AddressedTimedData {
            dst: value.options.next_hop(),
            data: value.data,
        }
    }
}

pub type PipelinePayload<Ts, Opts, NdId> = PipelineData<Ts, Vec<u8>, Opts, NdId>;
