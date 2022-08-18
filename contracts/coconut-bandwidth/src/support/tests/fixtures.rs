// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_bandwidth_contract_common::spend_credential::{SpendCredential, SpendCredentialData};
use cosmwasm_std::{Addr, Coin};

pub const TEST_MIX_DENOM: &str = "unym";

pub fn spend_credential_fixture(blinded_serial_number: &str) -> SpendCredential {
    SpendCredential::new(
        Coin::new(100, TEST_MIX_DENOM),
        blinded_serial_number.to_string(),
        Addr::unchecked("gateway_owner_addr"),
    )
}

pub fn spend_credential_data_fixture(blinded_serial_number: &str) -> SpendCredentialData {
    SpendCredentialData::new(
        Coin::new(100, TEST_MIX_DENOM),
        blinded_serial_number.to_string(),
        "gateway_owner_addr".to_string(),
    )
}
