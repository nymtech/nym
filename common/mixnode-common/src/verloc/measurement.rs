// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity;
use serde::{Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct AtomicVerlocResult {
    inner: Arc<RwLock<VerlocResult>>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct VerlocResult {
    total_tested: usize,
    #[serde(with = "humantime_serde")]
    run_started: Option<std::time::SystemTime>,
    #[serde(with = "humantime_serde")]
    run_finished: Option<std::time::SystemTime>,
    results: Vec<Verloc>,
}

impl AtomicVerlocResult {
    pub(crate) fn new() -> Self {
        AtomicVerlocResult {
            inner: Arc::new(RwLock::new(VerlocResult {
                total_tested: 0,
                run_started: None,
                run_finished: None,
                results: Vec::new(),
            })),
        }
    }

    pub(crate) async fn reset_results(&self, new_tested: usize) {
        let mut write_permit = self.inner.write().await;
        write_permit.total_tested = new_tested;
        write_permit.run_started = Some(std::time::SystemTime::now());
        write_permit.run_finished = None;
        write_permit.results = Vec::new()
    }

    pub(crate) async fn append_results(&self, mut new_data: Vec<Verloc>) {
        let mut write_permit = self.inner.write().await;
        write_permit.results.append(&mut new_data);
        // make sure the data always stays in order.
        // TODO: considering the front of the results is guaranteed to be sorted, should perhaps
        // a non-default sorting algorithm be used?
        write_permit.results.sort()
    }

    pub(crate) async fn finish_measurements(&self) {
        self.inner.write().await.run_finished = Some(std::time::SystemTime::now());
    }

    // Considering that on every read we will need to clone data regardless, let's make our
    // lives simpler and clone it here rather than deal with lifetime of the permit
    pub async fn clone_data(&self) -> VerlocResult {
        self.inner.read().await.clone()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct Verloc {
    #[serde(serialize_with = "serialize_identity_as_string")]
    pub identity: identity::PublicKey,
    pub latest_measurement: Option<Measurement>,
}

fn serialize_identity_as_string<S>(
    identity: &identity::PublicKey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&identity.to_base58_string())
}

impl Display for Verloc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(measurement) = self.latest_measurement {
            write!(f, "{} - {}", self.identity, measurement)
        } else {
            write!(f, "{} - COULD NOT MEASURE", self.identity)
        }
    }
}

impl PartialOrd for Verloc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Verloc {
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

impl Verloc {
    pub(crate) fn new(
        identity: identity::PublicKey,
        latest_measurement: Option<Measurement>,
    ) -> Self {
        Verloc {
            identity,
            latest_measurement,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct Measurement {
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub minimum: Duration,
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub mean: Duration,
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub maximum: Duration,
    #[serde(serialize_with = "humantime_serde::serialize")]
    pub standard_deviation: Duration,
}

impl Measurement {
    pub(crate) fn new(raw_results: &[Duration]) -> Self {
        let minimum = *raw_results.iter().min().expect("didn't get any results!");
        let maximum = *raw_results.iter().max().expect("didn't get any results!");

        let mean = Self::duration_mean(raw_results);
        let standard_deviation = Self::duration_standard_deviation(raw_results, mean);

        Measurement {
            minimum,
            mean,
            maximum,
            standard_deviation,
        }
    }

    fn duration_mean(data: &[Duration]) -> Duration {
        let sum = data.iter().sum::<Duration>();
        let count = data.len() as u32;

        sum / count
    }

    fn duration_standard_deviation(data: &[Duration], mean: Duration) -> Duration {
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

impl Display for Measurement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rtt min/avg/max/mdev = {:?} / {:?} / {:?} / {:?}",
            self.minimum, self.mean, self.maximum, self.standard_deviation
        )
    }
}

impl PartialOrd for Measurement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Measurement {
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
        let no_measurement = Verloc::new(some_identity, None);
        let low_min = Verloc::new(
            some_identity,
            Some(Measurement {
                minimum: Duration::from_millis(42),
                mean: Duration::from_millis(43),
                maximum: Duration::from_millis(44),
                standard_deviation: Duration::from_millis(45),
            }),
        );
        let higher_min = Verloc::new(
            some_identity,
            Some(Measurement {
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
