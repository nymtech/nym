// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, NodeId, NodeSubmission, Weights};
use cosmwasm_schema::cw_serde;

#[cfg(feature = "schema")]
use crate::types::{
    EpochMeasurementsPagedResponse, EpochPerformancePagedResponse,
    FullHistoricalPerformancePagedResponse, LastKnownEpochResponse, LastSubmission,
    NetworkMonitorResponse, NetworkMonitorsPagedResponse, NodeMeasurementsResponse,
    NodePerformancePagedResponse, NodePerformanceResponse, RetiredNetworkMonitorsPagedResponse,
    RewardingInputsResponse, RewardingScoreResponse, WeightsResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub mixnet_contract_address: String,
    pub authorised_network_monitors: Vec<String>,

    /// Weights in force from the creation epoch onwards.
    pub initial_weights: Weights,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin { admin: String },

    /// Attempt to submit measurements of a particular node for the current epoch
    Submit {
        epoch: EpochId,
        data: NodeSubmission,
    },

    /// Attempt to submit measurements of a batch of nodes, sorted by node id, for the current epoch
    BatchSubmit {
        epoch: EpochId,
        data: Vec<NodeSubmission>,
    },

    /// An admin method to replace the weights from the next epoch onwards
    UpdateWeights { weights: Weights },

    /// Attempt to authorise new network monitor for submitting performance data
    AuthoriseNetworkMonitor { address: String },

    /// Attempt to retire an existing network monitor and forbid it from submitting any future performance data
    RetireNetworkMonitor { address: String },

    /// An admin method to remove submitted node measurements. Used as an escape hatch should
    /// the data stored get too unwieldy.
    RemoveNodeMeasurements { epoch_id: EpochId, node_id: NodeId },

    /// An admin method to remove submitted nodes measurements. Used as an escape hatch should
    /// the data stored get too unwieldy. Note: it is expected to get called multiple times
    /// until the response indicates all the epoch data has been removed.
    RemoveEpochMeasurements { epoch_id: EpochId },
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(cosmwasm_schema::QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

    /// Returns per-kind medians and the score of particular node for exactly the provided epoch
    #[cfg_attr(feature = "schema", returns(NodePerformanceResponse))]
    NodePerformance { epoch_id: EpochId, node_id: NodeId },

    /// Returns historical performance for particular node, up to its last known epoch
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

    /// Returns everything rewarding uses for the node in the epoch: the resolved medians,
    /// the epoch they came from, the weights in force and the score
    #[cfg_attr(feature = "schema", returns(RewardingInputsResponse))]
    RewardingInputs { epoch_id: EpochId, node_id: NodeId },

    /// Returns only the score rewarding uses for the node in the epoch
    #[cfg_attr(feature = "schema", returns(RewardingScoreResponse))]
    RewardingScore { epoch_id: EpochId, node_id: NodeId },

    /// Returns the last epoch the node has any measurements for
    #[cfg_attr(feature = "schema", returns(LastKnownEpochResponse))]
    LastKnownEpoch { node_id: NodeId },

    /// Returns the weights in force at the provided epoch
    #[cfg_attr(feature = "schema", returns(WeightsResponse))]
    WeightsAt { epoch_id: EpochId },

    /// Returns the weights in force at the current mixnet epoch
    #[cfg_attr(feature = "schema", returns(WeightsResponse))]
    CurrentWeights {},

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

    /// Returns information regarding the latest submitted performance data
    #[cfg_attr(feature = "schema", returns(LastSubmission))]
    LastSubmittedMeasurement {},
}

#[cw_serde]
pub struct MigrateMsg {
    //
}
