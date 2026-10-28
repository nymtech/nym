// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Turning stored test runs into the per-node, per-epoch figures the orchestrator serves.
//!
//! Every rule about what a number MEANS lives here rather than in the storage layer, which only
//! keeps the rows, or in the HTTP layer, which only hands them out.

use crate::storage::models::{CompletedTestRun, MixnetEpochAggregate, TestKind};
use nym_network_monitor_orchestrator_requests::models::KindAggregate;
use std::collections::BTreeMap;

pub(crate) mod materialiser;

/// What a set of runs amounts to: the mean of their scores and how many runs it was taken over.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct WindowAggregate {
    /// Mean of the scores of the runs.
    pub(crate) score: f64,

    /// How many runs that mean was taken over. Never zero: no runs means no aggregate.
    pub(crate) samples: u32,
}

impl WindowAggregate {
    /// The mean of `scores`, or `None` when there are none.
    ///
    /// Each run weighs the same however many packets it sent, rather than the packet counts beneath
    /// the scores being pooled: that is how nym-api averages the same runs, which is what keeps the
    /// two figures comparable.
    fn mean(scores: &[f64]) -> Option<Self> {
        if scores.is_empty() {
            return None;
        }

        let total: f64 = scores.iter().sum();
        Some(WindowAggregate {
            score: total / scores.len() as f64,
            samples: scores.len() as u32,
        })
    }

    /// The aggregate over the runs behind both `self` and `other`, each weighted by its sample
    /// count. Equal to the mean over both sets of runs, since a mean and its count determine their
    /// sum.
    pub(crate) fn combine(self, other: WindowAggregate) -> Self {
        let samples = self.samples + other.samples;
        let total = self.score * f64::from(self.samples) + other.score * f64::from(other.samples);
        WindowAggregate {
            score: total / f64::from(samples),
            samples,
        }
    }
}

impl From<&MixnetEpochAggregate> for WindowAggregate {
    fn from(row: &MixnetEpochAggregate) -> Self {
        WindowAggregate {
            score: row.score,
            samples: row.samples as u32,
        }
    }
}

impl From<WindowAggregate> for KindAggregate {
    fn from(aggregate: WindowAggregate) -> Self {
        KindAggregate {
            score: aggregate.score,
            samples: aggregate.samples,
        }
    }
}

/// Each node's aggregate over one kind's runs, keyed by node id. A node without a run has no entry,
/// so it can never be read as one measured at zero.
///
/// A run that failed is scored like any other rather than dropped: an interface that sent nothing
/// rates zero, so unmeasurable never scores better than measurably broken.
pub(crate) fn aggregate_runs(runs: Vec<CompletedTestRun>) -> BTreeMap<i64, WindowAggregate> {
    let mut scores: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
    for run in runs {
        scores.entry(run.run.node_id).or_default().push(run.score());
    }

    scores
        .into_iter()
        .filter_map(|(node_id, scores)| Some((node_id, WindowAggregate::mean(&scores)?)))
        .collect()
}

/// One node's aggregates for one epoch as they are reported: both liveness kinds combined into a
/// single liveness figure, stress on its own.
///
/// Combining by sample count gives the mean over both kinds' runs, which is what nym-api computes,
/// both kinds' results landing in its one liveness series. Exact only while the two kinds share one
/// aggregation window, which [`AggregationWindows`](crate::orchestrator::config::AggregationWindows)
/// guarantees.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub(crate) struct ReportedAggregates {
    pub(crate) liveness: Option<WindowAggregate>,
    pub(crate) stress: Option<WindowAggregate>,
}

impl ReportedAggregates {
    /// Folds one node's stored aggregates, at most one per kind, into the reported figures.
    pub(crate) fn from_rows(rows: &[MixnetEpochAggregate]) -> Self {
        let mut reported = ReportedAggregates::default();
        for row in rows {
            let aggregate = WindowAggregate::from(row);
            match row.test_kind {
                TestKind::MixnodeLiveness | TestKind::GatewayLiveness => {
                    reported.liveness = Some(match reported.liveness {
                        Some(liveness) => liveness.combine(aggregate),
                        None => aggregate,
                    });
                }
                TestKind::MixnodeStress => reported.stress = Some(aggregate),
            }
        }
        reported
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{NewTestRun, minimal_measurement, minimal_test_run};
    use nym_network_monitor_orchestrator_requests::models::{
        InterfaceMeasurement, RunMeasurements,
    };
    use time::macros::datetime;

    /// A stress run against `node_id` that got `received` of its `sent` packets back.
    fn run(node_id: i64, sent: usize, received: usize) -> CompletedTestRun {
        CompletedTestRun {
            id: 0,
            run: minimal_test_run(node_id),
            measurements: RunMeasurements::MixnodeStress {
                mix_forwarding: InterfaceMeasurement {
                    packets_sent: sent,
                    packets_received: received,
                    ..minimal_measurement()
                },
            },
        }
    }

    fn row(test_kind: TestKind, score: f64, samples: i64) -> MixnetEpochAggregate {
        MixnetEpochAggregate {
            mixnet_epoch: 7,
            epoch_start: datetime!(2025-06-01 12:00:00 UTC),
            node_id: 1,
            test_kind,
            score,
            samples,
        }
    }

    // pooling the 53 packets would give 50/53, so a node that failed a short run would read as
    // almost perfect
    #[test]
    fn each_run_weighs_the_same_regardless_of_how_many_packets_it_sent() {
        let aggregates = aggregate_runs(vec![run(1, 50, 50), run(1, 3, 0)]);

        assert_eq!(
            aggregates[&1],
            WindowAggregate {
                score: 0.5,
                samples: 2
            }
        );
    }

    #[test]
    fn runs_are_aggregated_per_node() {
        let aggregates = aggregate_runs(vec![run(1, 10, 10), run(2, 10, 0)]);

        assert_eq!(aggregates.len(), 2);
        assert_eq!(aggregates[&1].score, 1.0);
        assert_eq!(aggregates[&2].score, 0.0);
    }

    // excluding it would let a node that fails every probe read the same as one never probed
    #[test]
    fn a_run_that_failed_pulls_the_aggregate_down() {
        let failed = CompletedTestRun {
            run: NewTestRun {
                error: Some("connection refused".to_string()),
                ..minimal_test_run(1)
            },
            ..run(1, 0, 0)
        };

        let aggregates =
            aggregate_runs(vec![run(1, 10, 10), run(1, 10, 10), run(1, 10, 10), failed]);

        assert_eq!(
            aggregates[&1],
            WindowAggregate {
                score: 0.75,
                samples: 4
            }
        );
    }

    // the mean over all four runs, not the mean of the two kinds' means (0.5)
    #[test]
    fn liveness_combines_both_kinds_by_sample_count() {
        let reported = ReportedAggregates::from_rows(&[
            row(TestKind::GatewayLiveness, 0.0, 1),
            row(TestKind::MixnodeLiveness, 1.0, 3),
        ]);

        assert_eq!(
            reported.liveness,
            Some(WindowAggregate {
                score: 0.75,
                samples: 4
            })
        );
        assert_eq!(reported.stress, None);
    }

    // absent rather than zero, so "not measured" never reads as "measured badly"
    #[test]
    fn a_kind_with_no_row_is_absent() {
        let reported = ReportedAggregates::from_rows(&[row(TestKind::MixnodeStress, 0.5, 3)]);

        assert_eq!(reported.liveness, None);
        assert_eq!(
            reported.stress,
            Some(WindowAggregate {
                score: 0.5,
                samples: 3
            })
        );
    }
}
