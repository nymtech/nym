// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod delegation;
pub mod error;
pub mod events;
mod gateway;
mod interval;
pub mod mixnode;
mod msg;
pub mod reward_params;
mod types;

pub const MIXNODE_DELEGATORS_PAGE_LIMIT: usize = 250;

pub use cosmwasm_std::{Addr, Coin};
pub use delegation::{
    Delegation, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
    PagedMixDelegationsResponse,
};
pub use gateway::{Gateway, GatewayBond, GatewayOwnershipResponse, PagedGatewayResponse};
pub use interval::Interval;
pub use mixnode::{
    Layer, MixNode, MixNodeBond, MixOwnershipResponse, PagedMixnodeResponse, RewardedSetNodeStatus,
};
pub use msg::*;
pub use types::*;

pub type U128 = fixed::types::U75F53;

fixed::const_fixed_from_int! {
    const ONE: U128 = 1;
}
