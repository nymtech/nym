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

use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct NodeResult {
    minimum: Duration,
    maximum: Duration,
    mean: Duration,
    standard_deviation: Duration,
}

impl NodeResult {
    pub(crate) fn new(raw_results: &[Duration]) -> Self {
        let minimum = *raw_results.iter().min().expect("didn't get any results!");
        let maximum = *raw_results.iter().max().expect("didn't get any results!");

        let mean = Self::duration_mean(&raw_results);
        let standard_deviation = Self::duration_standard_deviation(&raw_results, mean);

        NodeResult {
            minimum,
            maximum,
            mean,
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

impl Display for NodeResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rtt min/avg/max/mdev = {:?} / {:?} / {:?} / {:?}",
            self.minimum, self.mean, self.maximum, self.standard_deviation
        )
    }
}
