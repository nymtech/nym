// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod delegation;
mod gateway;
mod mixnode;
mod msg;
mod types;
pub mod error;

pub use cosmwasm_std::{Addr, Coin};
pub use delegation::{
    Delegation, PagedAllDelegationsResponse, PagedGatewayDelegationsResponse,
    PagedMixDelegationsResponse, PagedReverseGatewayDelegationsResponse,
    PagedReverseMixDelegationsResponse, RawDelegationData, UnpackedDelegation,
};
pub use gateway::{Gateway, GatewayBond, GatewayOwnershipResponse, PagedGatewayResponse};
pub use mixnode::{Layer, MixNode, MixNodeBond, MixOwnershipResponse, PagedMixnodeResponse};
pub use msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use types::{IdentityKey, IdentityKeyRef, LayerDistribution, SphinxKey, StateParams};

use std::sync::atomic::{AtomicU64, Ordering};
pub static CURRENT_BLOCK_HEIGHT: AtomicU64 = AtomicU64::new(0);

pub fn current_block_height() -> u64 {
    CURRENT_BLOCK_HEIGHT.load(Ordering::Relaxed)
}
