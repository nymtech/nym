// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{EpochId, Interval as ContractInterval, IntervalId};
use serde::{Deserialize, Serialize};

// TODO: ask @MS why we can't just use ContractInterval directly
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/Interval.ts")
)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Serialize)]
pub struct Interval {
    id: IntervalId,
    epochs_in_interval: u32,

    current_epoch_start_unix: i64,
    current_epoch_id: EpochId,
    epoch_length_seconds: u64,
    total_elapsed_epochs: EpochId,
}

impl From<ContractInterval> for Interval {
    fn from(contract_interval: ContractInterval) -> Self {
        Interval {
            id: contract_interval.current_interval_id(),
            epochs_in_interval: contract_interval.epochs_in_interval(),
            current_epoch_start_unix: contract_interval.current_epoch_start_unix_timestamp(),
            current_epoch_id: contract_interval.current_epoch_id(),
            epoch_length_seconds: contract_interval.epoch_length_secs(),
            total_elapsed_epochs: contract_interval.total_elapsed_epochs(),
        }
    }
}
