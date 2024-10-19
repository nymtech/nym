// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity::{self, serde_helpers::bs58_ed25519_pubkey};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MixingStats {
    #[serde(with = "time::serde::rfc3339")]
    pub update_time: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    pub previous_update_time: OffsetDateTime,

    pub received_since_startup: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub sent_since_startup: u64,

    // we know for sure we dropped those packets
    pub dropped_since_startup: u64,

    pub received_since_last_update: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub sent_since_last_update: u64,

    // we know for sure we dropped those packets
    pub dropped_since_last_update: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Session {
    pub duration_ms: u64,
    pub typ: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SessionStats {
    #[serde(with = "time::serde::rfc3339")]
    pub update_time: OffsetDateTime,

    pub unique_active_users: u32,

    pub sessions: Vec<Session>,

    pub sessions_started: u32,

    pub sessions_finished: u32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerlocStats {
    pub previous: VerlocResult,
    pub current: VerlocResult,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub enum VerlocResult {
    Data(VerlocResultData),
    MeasurementInProgress,
    #[default]
    Unavailable,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerlocResultData {
    pub nodes_tested: usize,

    #[serde(with = "time::serde::rfc3339")]
    pub run_started: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339::option")]
    pub run_finished: Option<OffsetDateTime>,

    pub results: Vec<VerlocNodeResult>,
}

impl Default for VerlocResultData {
    fn default() -> Self {
        VerlocResultData {
            nodes_tested: 0,
            run_started: OffsetDateTime::now_utc(),
            run_finished: None,
            results: vec![],
        }
    }
}

impl VerlocResultData {
    pub fn run_finished(&self) -> bool {
        self.run_finished.is_some()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerlocNodeResult {
    #[serde(with = "bs58_ed25519_pubkey")]
    pub node_identity: identity::PublicKey,

    pub latest_measurement: Option<VerlocMeasurement>,
}

impl VerlocNodeResult {
    pub fn new(
        node_identity: identity::PublicKey,
        latest_measurement: Option<VerlocMeasurement>,
    ) -> Self {
        VerlocNodeResult {
            node_identity,
            latest_measurement,
        }
    }
}

impl PartialOrd for VerlocNodeResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VerlocNodeResult {
    fn cmp(&self, other: &Self) -> Ordering {
        // if both have measurement, compare measurements
        // then if only one have measurement, prefer that one
        // completely ignore identity as it makes no sense to order by it
        if let Some(self_measurement) = &self.latest_measurement {
            if let Some(other_measurement) = &other.latest_measurement {
                self_measurement.cmp(other_measurement)
            } else {
                Ordering::Less
            }
        } else if other.latest_measurement.is_some() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerlocMeasurement {
    /// Minimum RTT duration it took to receive an echo packet.
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub minimum: Duration,

    /// Average RTT duration it took to receive the echo packets.
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub mean: Duration,

    /// Maximum RTT duration it took to receive an echo packet.
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub maximum: Duration,

    /// The standard deviation of the RTT duration it took to receive the echo packets.
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub standard_deviation: Duration,
}

impl VerlocMeasurement {
    pub fn new(raw_results: &[Duration]) -> Self {
        let minimum = raw_results.iter().min().copied().unwrap_or_default();
        let maximum = raw_results.iter().max().copied().unwrap_or_default();

        let mean = Self::duration_mean(raw_results);
        let standard_deviation = Self::duration_standard_deviation(raw_results, mean);

        VerlocMeasurement {
            minimum,
            mean,
            maximum,
            standard_deviation,
        }
    }

    fn duration_mean(data: &[Duration]) -> Duration {
        if data.is_empty() {
            return Default::default();
        }

        let sum = data.iter().sum::<Duration>();
        let count = data.len() as u32;

        sum / count
    }

    fn duration_standard_deviation(data: &[Duration], mean: Duration) -> Duration {
        if data.is_empty() {
            return Default::default();
        }

        let variance_micros = data
            .iter()
            .map(|&value| {
                // make sure we don't underflow
                let diff = if mean > value {
                    mean - value
                } else {
                    value - mean
                };
                // we don't need nanos precision
                let diff_micros = diff.as_micros();
                diff_micros * diff_micros
            })
            .sum::<u128>()
            / data.len() as u128;

        // we shouldn't really overflow as our differences shouldn't be larger than couple seconds at the worst possible case scenario
        let std_deviation_micros = (variance_micros as f64).sqrt() as u64;
        Duration::from_micros(std_deviation_micros)
    }
}

impl Display for VerlocMeasurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rtt min/avg/max/mdev = {} / {} / {} / {}",
            humantime::format_duration(self.minimum),
            humantime::format_duration(self.mean),
            humantime::format_duration(self.maximum),
            humantime::format_duration(self.standard_deviation)
        )
    }
}

impl PartialOrd for VerlocMeasurement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VerlocMeasurement {
    fn cmp(&self, other: &Self) -> Ordering {
        // minimum value is most important, then look at standard deviation, then mean and finally maximum
        let min_cmp = self.minimum.cmp(&other.minimum);
        if min_cmp != Ordering::Equal {
            return min_cmp;
        }
        let std_dev_cmp = self.standard_deviation.cmp(&other.standard_deviation);
        if std_dev_cmp != Ordering::Equal {
            return std_dev_cmp;
        }
        let std_dev_cmp = self.mean.cmp(&other.mean);
        if std_dev_cmp != Ordering::Equal {
            return std_dev_cmp;
        }
        self.maximum.cmp(&other.maximum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_vec_of_verlocs() {
        let some_identity =
            identity::PublicKey::from_base58_string("Be9wH7xuXBRJAuV1pC7MALZv6a61RvWQ3SypsNarqTt")
                .unwrap();
        let no_measurement = VerlocNodeResult::new(some_identity, None);
        let low_min = VerlocNodeResult::new(
            some_identity,
            Some(VerlocMeasurement {
                minimum: Duration::from_millis(42),
                mean: Duration::from_millis(43),
                maximum: Duration::from_millis(44),
                standard_deviation: Duration::from_millis(45),
            }),
        );
        let higher_min = VerlocNodeResult::new(
            some_identity,
            Some(VerlocMeasurement {
                minimum: Duration::from_millis(420),
                mean: Duration::from_millis(430),
                maximum: Duration::from_millis(440),
                standard_deviation: Duration::from_millis(450),
            }),
        );

        let mut vec_verloc = vec![no_measurement, low_min, no_measurement, higher_min];
        vec_verloc.sort();

        let expected_sorted = vec![low_min, higher_min, no_measurement, no_measurement];
        assert_eq!(expected_sorted, vec_verloc);
    }
}
