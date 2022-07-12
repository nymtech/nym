// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod constants;
pub mod delegation;
pub mod error;
pub mod events;
pub mod gateway;
mod interval;
pub mod mixnode;
mod msg;
pub mod pending_events;
pub mod reward_params;
pub mod rewarding;
mod types;

pub const MIXNODE_DELEGATORS_PAGE_LIMIT: usize = 250;

pub use contracts_common::types::*;
pub use cosmwasm_std::{Addr, Coin};
pub use delegation::{
    Delegation, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
    PagedMixNodeDelegationsResponse,
};
pub use gateway::{
    Gateway, GatewayBond, GatewayBondResponse, GatewayOwnershipResponse, PagedGatewayResponse,
};
pub use interval::{FullEpochId, Interval};
pub use mixnode::{
    Layer, MixNode, MixNodeBond, MixOwnershipResponse, MixnodeDetailsResponse,
    PagedMixnodeBondsResponse, RewardedSetNodeStatus,
};
pub use msg::*;
pub use types::*;

pub type U128 = fixed::types::U75F53;

fixed::const_fixed_from_int! {
    const ONE: U128 = 1;
}
