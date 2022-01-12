// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin, Event, Timestamp};

// event types
pub const WITHDRAW_EVENT_TYPE: &str = "vested_coins_withdraw";
pub const OWNERSHIP_TRANSFER_EVENT_TYPE: &str = "ownership_transfer";
pub const STAKING_ADDRESS_UPDATE_EVENT_TYPE: &str = "staking_address_update";
pub const NEW_PERIODIC_VESTING_ACCOUNT_EVENT_TYPE: &str = "new_periodic_vesting_account";

pub const VESTING_DELEGATION_EVENT_TYPE: &str = "vesting_delegation";
pub const VESTING_UNDELEGATION_EVENT_TYPE: &str = "vesting_undelegation";
pub const VESTING_GATEWAY_BONDING_EVENT_TYPE: &str = "vesting_gateway_bonding";
pub const VESTING_GATEWAY_UNBONDING_EVENT_TYPE: &str = "vesting_gateway_unbonding";
pub const VESTING_MIXNODE_BONDING_EVENT_TYPE: &str = "vesting_mixnode_bonding";
pub const VESTING_MIXNODE_UNBONDING_EVENT_TYPE: &str = "vesting_mixnode_unbonding";

pub const TRACK_MIXNODE_UNBOND_EVENT_TYPE: &str = "track_mixnode_unbond";
pub const TRACK_GATEWAY_UNBOND_EVENT_TYPE: &str = "track_gateway_unbond";
pub const TRACK_UNDELEGATION_EVENT_TYPE: &str = "track_undelegation";

// attributes that are used in multiple places
pub const OWNER_KEY: &str = "owner";
pub const AMOUNT_KEY: &str = "amount";

// event-specific attributes

// withdraw
pub const REMAINING_SPENDABLE_KEY: &str = "remaining_spendable";

// ownership transfer
pub const FROM_ACCOUNT_KEY: &str = "from";
pub const TO_ACCOUNT_KEY: &str = "to";
pub const NO_VALUE_VALUE: &str = "none";

// new vesting account
pub const START_TIME_KEY: &str = "start_time";
pub const STAKING_ADDRESS_KEY: &str = "staking_address";

// OPEN QUESTION: would it make sense to also emit amount of vesting/locked coins here?
// however, then it would require additional storage reads.
pub fn new_vested_coins_withdraw_event(
    address: &Addr,
    amount: &Coin,
    remaining_spendable: &Coin,
) -> Event {
    Event::new(WITHDRAW_EVENT_TYPE)
        .add_attribute(OWNER_KEY, address)
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(REMAINING_SPENDABLE_KEY, remaining_spendable.to_string())
}

pub fn new_ownership_transfer_event(from: &Addr, to: &Addr) -> Event {
    Event::new(OWNERSHIP_TRANSFER_EVENT_TYPE)
        .add_attribute(FROM_ACCOUNT_KEY, from)
        .add_attribute(TO_ACCOUNT_KEY, to)
}

pub fn new_staking_address_update_event(from: &Option<Addr>, to: &Option<Addr>) -> Event {
    let mut event = Event::new(OWNERSHIP_TRANSFER_EVENT_TYPE);

    if let Some(from) = from {
        event = event.add_attribute(FROM_ACCOUNT_KEY, from)
    } else {
        event = event.add_attribute(FROM_ACCOUNT_KEY, NO_VALUE_VALUE);
    }

    if let Some(to) = to {
        event = event.add_attribute(TO_ACCOUNT_KEY, to)
    } else {
        event = event.add_attribute(TO_ACCOUNT_KEY, NO_VALUE_VALUE);
    }

    event
}

pub fn new_periodic_vesting_account_event(
    owner_address: &Addr,
    amount: &Coin,
    staking_address: &Option<Addr>,
    start_time: Timestamp,
) -> Event {
    let mut event = Event::new(NEW_PERIODIC_VESTING_ACCOUNT_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner_address)
        .add_attribute(AMOUNT_KEY, amount.to_string());

    if let Some(staking_address) = staking_address {
        event = event.add_attribute(STAKING_ADDRESS_KEY, staking_address);
    }

    event.add_attribute(START_TIME_KEY, start_time.to_string())
}

// In most cases the events are rather barebone as there's no point in attaching
// bunch of data to them as it would be redundant. It is because in most cases when the event is emitted
// a call to the mixnet contract is made that throws another event with relevant attributes already attached.

pub fn new_vesting_gateway_bonding_event() -> Event {
    Event::new(VESTING_GATEWAY_BONDING_EVENT_TYPE)
}

pub fn new_vesting_gateway_unbonding_event() -> Event {
    Event::new(VESTING_GATEWAY_UNBONDING_EVENT_TYPE)
}

pub fn new_vesting_mixnode_bonding_event() -> Event {
    Event::new(VESTING_MIXNODE_BONDING_EVENT_TYPE)
}

pub fn new_vesting_mixnode_unbonding_event() -> Event {
    Event::new(VESTING_MIXNODE_UNBONDING_EVENT_TYPE)
}

pub fn new_vesting_delegation_event() -> Event {
    Event::new(VESTING_DELEGATION_EVENT_TYPE)
}

pub fn new_vesting_undelegation_event() -> Event {
    Event::new(VESTING_UNDELEGATION_EVENT_TYPE)
}

pub fn new_track_mixnode_unbond_event() -> Event {
    Event::new(TRACK_MIXNODE_UNBOND_EVENT_TYPE)
}

pub fn new_track_gateway_unbond_event() -> Event {
    Event::new(TRACK_GATEWAY_UNBOND_EVENT_TYPE)
}

pub fn new_track_undelegation_event() -> Event {
    Event::new(TRACK_UNDELEGATION_EVENT_TYPE)
}
