// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Env};
use nym_contracts_common::Percent;

pub type EpochId = u32;
pub type NodeId = u32;

#[cw_serde]
pub struct NetworkMonitorDetails {
    pub address: Addr,
    pub authorised_by: Addr,
    pub authorised_at_height: u64,
}

impl NetworkMonitorDetails {
    pub fn retire(self, env: &Env, sender: &Addr) -> RetiredNetworkMonitor {
        RetiredNetworkMonitor {
            details: self,
            retired_by: sender.clone(),
            retired_at_height: env.block.height,
        }
    }
}

#[cw_serde]
pub struct RetiredNetworkMonitor {
    pub details: NetworkMonitorDetails,
    pub retired_by: Addr,
    pub retired_at_height: u64,
}

#[cw_serde]
#[derive(Copy)]
pub struct NodePerformance {
    #[serde(rename = "n")]
    pub node_id: NodeId,

    // note: value is rounded to 2 decimal places.
    #[serde(rename = "p")]
    pub performance: Percent,
}

#[cw_serde]
pub struct NetworkMonitorSubmissionMetadata {
    pub last_submitted_epoch_id: EpochId,
    pub last_submitted_node_id: NodeId,
}

// the internal values are always sorted
#[cw_serde]
pub struct NodeResults(Vec<Percent>);

impl NodeResults {
    pub fn new(initial: Percent) -> NodeResults {
        NodeResults(vec![initial.round_to_two_decimal_places()])
    }

    // ASSUMPTION: number of NM will be relatively small, so loading the whole vector of values
    // to insert new one and resave is cheap
    pub fn insert_new(&mut self, result: Percent) {
        let result = result.round_to_two_decimal_places();
        let pos = self.0.binary_search(&result).unwrap_or_else(|e| e);
        self.0.insert(pos, result);
    }

    // SAFETY: there are no codepaths that allow constructing empty struct
    pub fn median(&self) -> Percent {
        let len = self.0.len();
        if len % 2 == 1 {
            // odd number of elements: return the middle one
            self.0[len / 2]
        } else {
            // even number: average the two middle elements
            let mid1 = self.0[len / 2 - 1];
            let mid2 = self.0[len / 2];
            mid1.average(&mid2).round_to_two_decimal_places()
        }
    }

    pub fn inner(&self) -> &[Percent] {
        &self.0
    }
}

#[cw_serde]
pub struct NodePerformanceResponse {
    pub performance: Option<Percent>,
}

#[cw_serde]
pub struct NodeMeasurementsResponse {
    pub measurements: Option<NodeResults>,
}

#[cw_serde]
#[derive(Copy)]
pub struct EpochNodePerformance {
    pub epoch: EpochId,
    pub performance: Option<Percent>,
}

#[cw_serde]
pub struct NodePerformancePagedResponse {
    pub node_id: NodeId,
    pub performance: Vec<EpochNodePerformance>,
    pub start_next_after: Option<EpochId>,
}

#[cw_serde]
pub struct EpochPerformancePagedResponse {
    pub epoch_id: EpochId,
    pub performance: Vec<NodePerformance>,
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
pub struct NodeMeasurement {
    pub node_id: NodeId,
    pub measurements: NodeResults,
}

#[cw_serde]
pub struct EpochMeasurementsPagedResponse {
    pub epoch_id: EpochId,
    pub measurements: Vec<NodeMeasurement>,
    pub start_next_after: Option<NodeId>,
}

#[cw_serde]
#[derive(Copy)]
pub struct HistoricalPerformance {
    pub epoch_id: EpochId,
    pub node_id: NodeId,
    pub performance: Percent,
}

#[cw_serde]
pub struct FullHistoricalPerformancePagedResponse {
    pub performance: Vec<HistoricalPerformance>,
    pub start_next_after: Option<(EpochId, NodeId)>,
}

#[cw_serde]
pub struct NetworkMonitorInformation {
    pub details: NetworkMonitorDetails,
    pub current_submission_metadata: NetworkMonitorSubmissionMetadata,
}

#[cw_serde]
pub struct NetworkMonitorResponse {
    pub info: Option<NetworkMonitorInformation>,
}

#[cw_serde]
pub struct NetworkMonitorsPagedResponse {
    pub info: Vec<NetworkMonitorInformation>,
    pub start_next_after: Option<String>,
}

#[cw_serde]
pub struct RetiredNetworkMonitorsPagedResponse {
    pub info: Vec<RetiredNetworkMonitor>,
    pub start_next_after: Option<String>,
}

#[cw_serde]
pub struct RemoveEpochMeasurementsResponse {
    pub additional_entries_to_remove_remaining: bool,
}

#[cw_serde]
#[derive(Default)]
pub struct BatchSubmissionResult {
    pub accepted_scores: u64,
    pub non_existent_nodes: Vec<NodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(raw: impl AsRef<str>) -> Percent {
        raw.as_ref().parse().unwrap()
    }

    fn ps(raw: &[&str]) -> Vec<Percent> {
        raw.iter().map(p).collect()
    }

    #[test]
    fn node_results_insertion() {
        let initial = NodeResults::new(p("0.5"));

        let mut smaller = initial.clone();
        let mut greater = initial.clone();

        smaller.insert_new(p("0.4"));
        greater.insert_new(p("0.6"));

        assert_eq!(smaller.0, ps(&["0.4", "0.5"]));
        assert_eq!(greater.0, ps(&["0.5", "0.6"]));

        let mut another = NodeResults(ps(&["0.1", "0.4", "0.5", "0.6", "0.6", "1.0"]));
        another.insert_new(p("0.6"));
        another.insert_new(p("0.2"));
        another.insert_new(p("0.7"));
        another.insert_new(p("0.3"));
        another.insert_new(p("0.3"));
        another.insert_new(p("0.55"));

        assert_eq!(
            another.0,
            ps(&[
                "0.1", "0.2", "0.3", "0.3", "0.4", "0.5", "0.55", "0.6", "0.6", "0.6", "0.7", "1.0"
            ])
        );
    }

    #[test]
    fn node_results_median() {
        let results = NodeResults(ps(&["0.1"]));
        assert_eq!(results.median(), p("0.1"));

        let results = NodeResults(ps(&["0.1", "0.2"]));
        assert_eq!(results.median(), p("0.15"));

        let results = NodeResults(ps(&["0.1", "0.2", "0.3"]));
        assert_eq!(results.median(), p("0.2"));

        let results = NodeResults(ps(&["0.1", "0.2", "0.3", "0.4"]));
        assert_eq!(results.median(), p("0.25"));

        let results = NodeResults(ps(&["0.1", "0.2", "0.3", "0.4", "0.5"]));
        assert_eq!(results.median(), p("0.3"));

        let results = NodeResults(ps(&["0", "0", "1", "1", "1", "1", "1"]));
        assert_eq!(results.median(), p("1"));
    }
}
