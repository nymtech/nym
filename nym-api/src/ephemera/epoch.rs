// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::nyxd;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use log::info;
use nym_mixnet_contract_common::Interval as RewardsInterval;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tokio::time::Interval;

//It synchronizes rewarding across simulated Nym-APIs and the "Smart Contract".
//Maintained by the "Smart Contract"
#[derive(Debug)]
pub struct Epoch {
    pub start_time: DateTime<Utc>,
    pub interval: Interval,
    pub epoch_start_id: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EpochInfo {
    pub epoch_id: u64,
    pub start_time: i64,
    pub duration: u64,
}

impl EpochInfo {
    pub(crate) fn new(epoch_id: u64, start_time: i64, duration: u64) -> Self {
        Self {
            epoch_id,
            start_time,
            duration,
        }
    }
}

impl From<RewardsInterval> for Epoch {
    fn from(value: RewardsInterval) -> Self {
        Epoch::new(EpochInfo::new(
            value.current_epoch_id() as u64,
            value.current_epoch_start().unix_timestamp(),
            value.epoch_length_secs(),
        ))
    }
}

impl Epoch {
    pub fn new(info: EpochInfo) -> Self {
        info!("New epoch duration");
        let start_time = NaiveDateTime::from_timestamp_opt(info.start_time, 0)
            .ok_or("Invalid start time")
            .unwrap();
        let start_time: DateTime<Utc> = DateTime::from_naive_utc_and_offset(start_time, Utc);
        let interval = tokio::time::interval(std::time::Duration::from_secs(info.duration));
        Self {
            start_time,
            interval,
            epoch_start_id: start_time.minute().into(),
        }
    }

    pub fn current_epoch_numer(&self) -> u64 {
        let since_start = (Utc::now().timestamp() - self.start_time.timestamp()) as u64;
        let nr = since_start / self.interval.period().as_secs();
        nr + self.epoch_start_id
    }

    pub(crate) async fn request_epoch(nyxd_client: nyxd::Client) -> anyhow::Result<Epoch> {
        let interval = nyxd_client.get_current_interval().await?.interval;
        Ok(Epoch::from(interval))
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Epoch {{ start_time: {}, interval: {:?}, epoch_start_id: {} }}",
            self.start_time,
            self.interval.period(),
            self.epoch_start_id
        )
    }
}
