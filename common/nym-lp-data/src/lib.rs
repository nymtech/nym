// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

pub mod clients;
pub mod mixnodes;

pub struct TimedData<Ts, D> {
    pub timestamp: Ts,
    pub data: D,
}

impl<Ts, D> Debug for TimedData<Ts, D>
where
    D: Debug,
    Ts: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TimedData {{")?;
        writeln!(f, "    data:")?;
        let data_debug = format!("{:#?}", &self.data);
        for line in data_debug.lines() {
            writeln!(f, "        {}", line)?;
        }
        writeln!(f, "    timestamp: {:#?},", &self.timestamp)?;
        write!(f, "}}")
    }
}

impl<Ts, D> TimedData<Ts, D> {
    pub fn new(timestamp: Ts, data: D) -> Self {
        TimedData { timestamp, data }
    }
    pub fn data_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(D) -> D,
    {
        TimedData {
            data: op(self.data),
            timestamp: self.timestamp,
        }
    }

    pub fn ts_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        TimedData {
            data: self.data,
            timestamp: op(self.timestamp),
        }
    }
}

/// Helper type to erase the Vec<u8> parameters
pub type TimedPayload<Ts> = TimedData<Ts, Vec<u8>>;
