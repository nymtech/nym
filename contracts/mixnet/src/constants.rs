// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, Uint128};
use mixnet_contract_common::{
    NodeCostParams, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_contracts_common::Percent;

/// Constant specifying minimum of coin amount required to bond a node
pub const INITIAL_PLEDGE_AMOUNT: Uint128 = Uint128::new(100_000_000);

pub fn default_node_costs<S: Into<String>>(rewarding_denom: S) -> NodeCostParams {
    // safety: our hardcoded PM value is a valid percent
    #[allow(clippy::unwrap_used)]
    NodeCostParams {
        profit_margin_percent: Percent::from_percentage_value(DEFAULT_PROFIT_MARGIN_PERCENT)
            .unwrap(),
        interval_operating_cost: Coin::new(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, rewarding_denom),
    }
}

// retrieval limits
// TODO: those would need to be empirically verified whether they're not way too small or way too high
pub const GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const GATEWAY_BOND_MAX_RETRIEVAL_LIMIT: u32 = 100;

pub const MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const MIXNODE_BOND_MAX_RETRIEVAL_LIMIT: u32 = 100;

pub const NYM_NODE_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const NYM_NODE_BOND_MAX_RETRIEVAL_LIMIT: u32 = 100;

pub const MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT: u32 = 75;
pub const NYM_NODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const NYM_NODE_DETAILS_MAX_RETRIEVAL_LIMIT: u32 = 75;

pub const UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT: u32 = 200;
pub const UNBONDED_NYM_NODES_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const UNBONDED_NYM_NODES_MAX_RETRIEVAL_LIMIT: u32 = 200;

pub const DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT: u32 = 500;

pub const EPOCH_EVENTS_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const EPOCH_EVENTS_MAX_RETRIEVAL_LIMIT: u32 = 100;

pub const INTERVAL_EVENTS_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const INTERVAL_EVENTS_MAX_RETRIEVAL_LIMIT: u32 = 100;

// storage keys
pub const DELEGATION_PK_NAMESPACE: &str = "dl";
pub const DELEGATION_OWNER_IDX_NAMESPACE: &str = "dlo";
pub const DELEGATION_MIXNODE_IDX_NAMESPACE: &str = "dlm";

pub const GATEWAYS_PK_NAMESPACE: &str = "gt";
pub const GATEWAYS_OWNER_IDX_NAMESPACE: &str = "gto";

pub const CURRENT_EPOCH_STATUS_KEY: &str = "ces";
pub const CURRENT_INTERVAL_KEY: &str = "ci";
pub const EPOCH_EVENT_ID_COUNTER_KEY: &str = "eic";
pub const INTERVAL_EVENT_ID_COUNTER_KEY: &str = "iic";
pub const PENDING_EPOCH_EVENTS_NAMESPACE: &str = "pee";
pub const PENDING_INTERVAL_EVENTS_NAMESPACE: &str = "pie";

pub const LAST_EPOCH_EVENT_ID_KEY: &str = "lee";
pub const LAST_INTERVAL_EVENT_ID_KEY: &str = "lie";

pub const ADMIN_STORAGE_KEY: &str = "admin";
pub const CONTRACT_STATE_KEY: &str = "state";

pub const NYMNODE_ROLES_ASSIGNMENT_NAMESPACE: &str = "roles";
pub const NYMNODE_REWARDED_SET_METADATA_NAMESPACE: &str = "roles_metadata";
pub const NYMNODE_ACTIVE_ROLE_ASSIGNMENT_KEY: &str = "active_roles";

pub const NODE_ID_COUNTER_KEY: &str = "nic";
pub const PENDING_MIXNODE_CHANGES_NAMESPACE: &str = "pmc";
pub const MIXNODES_PK_NAMESPACE: &str = "mnn";
pub const MIXNODES_OWNER_IDX_NAMESPACE: &str = "mno";
pub const MIXNODES_IDENTITY_IDX_NAMESPACE: &str = "mni";
pub const MIXNODES_SPHINX_IDX_NAMESPACE: &str = "mns";

pub const PENDING_NYMNODE_CHANGES_NAMESPACE: &str = "pnc";
pub const NYMNODE_PK_NAMESPACE: &str = "nn";
pub const NYMNODE_OWNER_IDX_NAMESPACE: &str = "nno";
pub const NYMNODE_IDENTITY_IDX_NAMESPACE: &str = "nni";

pub const UNBONDED_MIXNODES_PK_NAMESPACE: &str = "ubm";
pub const UNBONDED_MIXNODES_OWNER_IDX_NAMESPACE: &str = "umo";
pub const UNBONDED_MIXNODES_IDENTITY_IDX_NAMESPACE: &str = "umi";

pub const UNBONDED_NYMNODE_PK_NAMESPACE: &str = "ubnn";
pub const UNBONDED_NYMNODE_OWNER_IDX_NAMESPACE: &str = "ubno";
pub const UNBONDED_NYMNODE_IDENTITY_IDX_NAMESPACE: &str = "ubni";

pub const CUMULATIVE_EPOCH_WORK_KEY: &str = "cumulative_epoch_work";
pub const REWARDING_PARAMS_KEY: &str = "rparams";
pub const PENDING_REWARD_POOL_KEY: &str = "prp";
pub const MIXNODES_REWARDING_PK_NAMESPACE: &str = "mnr";
pub const NYMNODE_REWARDING_PK_NAMESPACE: &str = MIXNODES_REWARDING_PK_NAMESPACE;

pub const SIGNING_NONCES_NAMESPACE: &str = "sn";

// temporary storage keys created for the transition period:
pub const LEGACY_GATEWAY_ID_NAMESPACE: &str = "lgidr";
