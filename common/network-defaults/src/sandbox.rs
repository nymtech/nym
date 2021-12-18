// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const BECH32_PREFIX: &str = "nymt";
pub const DENOM: &str = "unymt";

pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "nymt14hj2tavq8fpesdwxxcu44rty3hh90vhuysqrsr";
pub const DEFAULT_VESTING_CONTRACT_ADDRESS: &str = "nymt1nc5tatafv6eyq7llkr2gv50ff9e22mnfp9pc5s";
pub const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub const REWARDING_VALIDATOR_ADDRESS: &str = "nymt17zujduc46wvkwvp6f062mm5xhr7jc3fewvqu9e";
pub const ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("9fEE3e28c17dbB87310A51F13C4fbf4331A6f102");

// Name of the event triggered by the eth contract. If the event name is changed,
// this would also need to be changed; It is currently tested against the json abi
pub const ETH_EVENT_NAME: &str = "Burned";
pub const ETH_BURN_FUNCTION_NAME: &str = "burnTokenForAccessCode";
