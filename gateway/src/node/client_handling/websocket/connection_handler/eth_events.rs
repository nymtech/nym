// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use web3::{contract::Contract, transports::Http};

pub fn verify_eth_events(_contract: &Contract<Http>) -> bool {
    true
}
