// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::UninitialisedCache;
use nym_mixnet_contract_common::NodeId;
use nym_node_requests::api::client::NymNodeApiClientError;
use thiserror::Error;

pub(crate) mod cache;
pub(crate) mod provider;
mod query_helpers;
pub(crate) mod refresh;

#[derive(Debug, Error)]
pub enum NodeDescribeCacheError {
    #[error("contract cache hasn't been initialised")]
    UninitialisedContractCache {
        #[from]
        source: UninitialisedCache,
    },

    #[error(transparent)]
    ClientRetrievalFailure(#[from] nym_node_requests::error::Error),

    #[error("failed to retrieve host information of node {node_id} - this is most likely a bug")]
    NoHostInformationAvailable { node_id: NodeId, host: String },

    #[error("failed to query node {node_id}: {source}")]
    ApiFailure {
        node_id: NodeId,

        #[source]
        source: Box<NymNodeApiClientError>,
    },

    #[error("node {node_id} is announcing an illegal ip address")]
    IllegalIpAddress { node_id: NodeId },
}
