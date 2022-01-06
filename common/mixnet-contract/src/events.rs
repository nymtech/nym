// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Delegation, IdentityKeyRef, Layer};
use cosmwasm_std::{Addr, Coin, Event};

// event types
pub const REWARDING_EVENT_TYPE: &str = "rewarding";
pub const DELEGATION_EVENT_TYPE: &str = "delegation";
pub const UNDELEGATION_EVENT_TYPE: &str = "undelegation";
pub const GATEWAY_BONDING_EVENT_TYPE: &str = "gateway_bonding";
pub const GATEWAY_UNBONDING_EVENT_TYPE: &str = "gateway_unbonding";
pub const MIXNODE_BONDING_EVENT_TYPE: &str = "mixnode_bonding";
pub const MIXNODE_UNBONDING_EVENT_TYPE: &str = "mixnode_unbonding";

// attributes that are used in multiple places
pub const OWNER_KEY: &str = "owner";
pub const AMOUNT_KEY: &str = "amount";
pub const PROXY_KEY: &str = "proxy";

// event-specific attributes

// delegation/undelegation
pub const DELEGATOR_KEY: &str = "delegator";
pub const DELEGATION_TARGET_KEY: &str = "delegation_target";
pub const DELEGATION_HEIGHT_KEY: &str = "delegation_latest_block_height";

// bonding/unbonding
pub const NODE_IDENTITY_KEY: &str = "identity";
pub const ASSIGNED_LAYER_KEY: &str = "assigned_layer";

pub fn new_delegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    mix_identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(DELEGATION_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
}

pub fn new_undelegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    old_delegation: &Delegation,
    mix_identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(UNDELEGATION_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, old_delegation.amount.to_string())
        .add_attribute(
            DELEGATION_HEIGHT_KEY,
            old_delegation.block_height.to_string(),
        )
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
}

pub fn new_gateway_bonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(GATEWAY_BONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_gateway_unbonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(GATEWAY_UNBONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_mixnode_bonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
    assigned_layer: Layer,
) -> Event {
    let mut event = Event::new(MIXNODE_BONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(ASSIGNED_LAYER_KEY, assigned_layer)
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_mixnode_unbonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(MIXNODE_UNBONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}
