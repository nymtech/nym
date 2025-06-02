// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, NodeId, NodePerformance};
use cosmwasm_schema::cw_serde;

#[cfg(feature = "schema")]
use crate::types::{
    EpochMeasurementsPagedResponse, EpochPerformancePagedResponse,
    FullHistoricalPerformancePagedResponse, NetworkMonitorResponse, NetworkMonitorsPagedResponse,
    NodeMeasurementsResponse, NodePerformancePagedResponse, NodePerformanceResponse,
    RetiredNetworkMonitorsPagedResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub mixnet_contract_address: String,
    pub authorised_network_monitors: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Attempt to submit performance data of a particular node for given epoch
    Submit {
        epoch: EpochId,
        data: NodePerformance,
    },

    /// Attempt to submit performance data of a batch of nodes for given epoch
    BatchSubmit {
        epoch: EpochId,
        data: Vec<NodePerformance>,
    },

    /// Attempt to authorise new network monitor for submitting performance data
    AuthoriseNetworkMonitor { address: String },

    /// Attempt to retire an existing network monitor and forbid it from submitting any future performance data
    RetireNetworkMonitor { address: String },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    /// Returns performance of particular node for the provided epoch
    #[cfg_attr(feature = "schema", returns(NodePerformanceResponse))]
    NodePerformance { epoch_id: EpochId, node_id: NodeId },

    /// Returns historical performance for particular node
    #[cfg_attr(feature = "schema", returns(NodePerformancePagedResponse))]
    NodePerformancePaged {
        node_id: NodeId,
        start_after: Option<EpochId>,
        limit: Option<u32>,
    },

    /// Returns all submitted measurements for the particular node
    #[cfg_attr(feature = "schema", returns(NodeMeasurementsResponse))]
    NodeMeasurements { epoch_id: EpochId, node_id: NodeId },

    /// Returns (paged) measurements for particular epoch
    #[cfg_attr(feature = "schema", returns(EpochMeasurementsPagedResponse))]
    EpochMeasurementsPaged {
        epoch_id: EpochId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    },

    /// Returns (paged) performance for particular epoch
    #[cfg_attr(feature = "schema", returns(EpochPerformancePagedResponse))]
    EpochPerformancePaged {
        epoch_id: EpochId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    },

    /// Returns full (paged) historical performance of the whole network
    #[cfg_attr(feature = "schema", returns(FullHistoricalPerformancePagedResponse))]
    FullHistoricalPerformancePaged {
        start_after: Option<(EpochId, NodeId)>,
        limit: Option<u32>,
    },

    /// Returns information about particular network monitor
    #[cfg_attr(feature = "schema", returns(NetworkMonitorResponse))]
    NetworkMonitor { address: String },

    /// Returns information about all network monitors
    #[cfg_attr(feature = "schema", returns(NetworkMonitorsPagedResponse))]
    NetworkMonitorsPaged {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns information about all retired network monitors
    #[cfg_attr(feature = "schema", returns(RetiredNetworkMonitorsPagedResponse))]
    RetiredNetworkMonitorsPaged {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {
    //
}
