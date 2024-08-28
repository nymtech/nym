// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Uint128;

/// Constant specifying minimum of coin amount required to bond a gateway
pub const INITIAL_GATEWAY_PLEDGE_AMOUNT: Uint128 = Uint128::new(100_000_000);

/// Constant specifying minimum of coin amount required to bond a mixnode
pub const INITIAL_MIXNODE_PLEDGE_AMOUNT: Uint128 = Uint128::new(100_000_000);

// retrieval limits
// TODO: those would need to be empirically verified whether they're not way too small or way too high
pub const GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const GATEWAY_BOND_MAX_RETRIEVAL_LIMIT: u32 = 150;

pub const MIXNODE_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const MIXNODE_BOND_MAX_RETRIEVAL_LIMIT: u32 = 150;

pub const MIXNODE_DETAILS_DEFAULT_RETRIEVAL_LIMIT: u32 = 75;
pub const MIXNODE_DETAILS_MAX_RETRIEVAL_LIMIT: u32 = 100;

pub const UNBONDED_MIXNODES_DEFAULT_RETRIEVAL_LIMIT: u32 = 250;
pub const UNBONDED_MIXNODES_MAX_RETRIEVAL_LIMIT: u32 = 300;

pub const DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT: u32 = 250;
pub const DELEGATION_PAGE_MAX_RETRIEVAL_LIMIT: u32 = 300;

pub const EPOCH_EVENTS_DEFAULT_RETRIEVAL_LIMIT: u32 = 200;
pub const EPOCH_EVENTS_MAX_RETRIEVAL_LIMIT: u32 = 250;

pub const INTERVAL_EVENTS_DEFAULT_RETRIEVAL_LIMIT: u32 = 200;
pub const INTERVAL_EVENTS_MAX_RETRIEVAL_LIMIT: u32 = 250;

pub const REWARDED_SET_DEFAULT_RETRIEVAL_LIMIT: u32 = 500;
pub const REWARDED_SET_MAX_RETRIEVAL_LIMIT: u32 = 1000;

pub const FAMILIES_DEFAULT_RETRIEVAL_LIMIT: u32 = 10;
pub const FAMILIES_MAX_RETRIEVAL_LIMIT: u32 = 20;

// storage keys
pub const DELEGATION_PK_NAMESPACE: &str = "dl";
pub const DELEGATION_OWNER_IDX_NAMESPACE: &str = "dlo";
pub const DELEGATION_MIXNODE_IDX_NAMESPACE: &str = "dlm";

pub const GATEWAYS_PK_NAMESPACE: &str = "gt";
pub const GATEWAYS_OWNER_IDX_NAMESPACE: &str = "gto";

pub const REWARDED_SET_KEY: &str = "rs";
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

pub const LAYER_DISTRIBUTION_KEY: &str = "layers";
pub const NODE_ID_COUNTER_KEY: &str = "nic";
pub const PENDING_MIXNODE_CHANGES_NAMESPACE: &str = "pmc";
pub const MIXNODES_PK_NAMESPACE: &str = "mnn";
pub const MIXNODES_OWNER_IDX_NAMESPACE: &str = "mno";
pub const MIXNODES_IDENTITY_IDX_NAMESPACE: &str = "mni";
pub const MIXNODES_SPHINX_IDX_NAMESPACE: &str = "mns";

pub const UNBONDED_MIXNODES_PK_NAMESPACE: &str = "ubm";
pub const UNBONDED_MIXNODES_OWNER_IDX_NAMESPACE: &str = "umo";
pub const UNBONDED_MIXNODES_IDENTITY_IDX_NAMESPACE: &str = "umi";

pub const REWARDING_PARAMS_KEY: &str = "rparams";
pub const PENDING_REWARD_POOL_KEY: &str = "prp";
pub const MIXNODES_REWARDING_PK_NAMESPACE: &str = "mnr";

pub const FAMILIES_INDEX_NAMESPACE: &str = "faml2";
pub const FAMILIES_MAP_NAMESPACE: &str = "fam2";
pub const MEMBERS_MAP_NAMESPACE: &str = "memb2";

pub const SIGNING_NONCES_NAMESPACE: &str = "sn";
