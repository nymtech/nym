use mixnet_contract_common::Interval;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/Epoch.ts")
)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Epoch {
    id: u32,
    start: i64,
    end: i64,
    duration_seconds: u64,
}

impl From<Interval> for Epoch {
    fn from(interval: Interval) -> Self {
        Self {
            id: interval.id(),
            start: interval.start_unix_timestamp(),
            end: interval.end_unix_timestamp(),
            duration_seconds: interval.length().as_secs(),
        }
    }
}
