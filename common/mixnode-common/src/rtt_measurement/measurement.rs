// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::identity;
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Verloc {
    identity: identity::PublicKey,
    latest_measurement: Option<Measurement>,
}

impl Display for Verloc {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let identity = self.identity.to_base58_string();
        if let Some(measurement) = self.latest_measurement {
            write!(f, "{} - {}", identity, measurement)
        } else {
            write!(f, "{} - COULD NOT MEASURE", identity)
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Measurement {
    minimum: Duration,
    mean: Duration,
    maximum: Duration,
    standard_deviation: Duration,
}

impl Measurement {
    pub(crate) fn new(raw_results: &[Duration]) -> Self {
        let minimum = *raw_results.iter().min().expect("didn't get any results!");
        let maximum = *raw_results.iter().max().expect("didn't get any results!");

        let mean = Self::duration_mean(&raw_results);
        let standard_deviation = Self::duration_standard_deviation(&raw_results, mean);

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
