// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;

pub(crate) struct PerformanceContractCacheData {
    // what about keeping data for different epochs?
    pub(crate) median_performance: HashMap<NodeId, Performance>,
}

// what about another wrapper
// provider gives us EpochId, HashMap<NodeId, Performance>
// which is pushed to another struct that has HashMap<EpochId, HashMap<NodeId, Performance>>
// and controls purging and whatnot
