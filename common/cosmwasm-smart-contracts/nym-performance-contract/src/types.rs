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
        NodeResults(vec![initial])
    }

    // ASSUMPTION: number of NM will be relatively small, so loading the whole vector of values
    // to insert new one and resave is cheap
    pub fn insert_new(&mut self, result: Percent) {
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
}
