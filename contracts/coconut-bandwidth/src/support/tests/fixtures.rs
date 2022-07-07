// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::MIX_DENOM;
use cosmwasm_std::{Addr, Coin};

use crate::storage::SpendCredential;

pub fn spend_credential_fixture(blinded_serial_number: &str) -> SpendCredential {
    SpendCredential::new(
        Coin::new(100, MIX_DENOM.base),
        blinded_serial_number.to_string(),
        Addr::unchecked("gateway_owner_addr"),
    )
}
