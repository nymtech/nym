// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::setup::{MIXNET_OWNER, MIX_DENOM, REWARDING_VALIDATOR, VESTING_OWNER};
use cosmwasm_std::{coin, coins, Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[allow(unused)]
pub fn mixnet_owner() -> Addr {
    Addr::unchecked(MIXNET_OWNER)
}

#[allow(unused)]
pub fn vesting_owner() -> Addr {
    Addr::unchecked(VESTING_OWNER)
}

#[allow(unused)]
pub fn rewarding_validator() -> Addr {
    Addr::unchecked(REWARDING_VALIDATOR)
}

#[allow(unused)]
pub fn mix_coins(amount: u128) -> Vec<Coin> {
    coins(amount, MIX_DENOM)
}

#[allow(unused)]
pub fn mix_coin(amount: u128) -> Coin {
    coin(amount, MIX_DENOM)
}

#[allow(unused)]
pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

#[allow(unused)]
pub fn mixnet_contract_wrapper() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            mixnet_contract::contract::execute,
            mixnet_contract::contract::instantiate,
            mixnet_contract::contract::query,
        )
        .with_migrate(mixnet_contract::contract::migrate),
    )
}

pub fn vesting_contract_wrapper() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            vesting_contract::contract::execute,
            vesting_contract::contract::instantiate,
            vesting_contract::contract::query,
        )
        .with_migrate(vesting_contract::contract::migrate),
    )
}
