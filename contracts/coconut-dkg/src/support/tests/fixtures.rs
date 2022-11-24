// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::verification_key::ContractVKShare;
use cosmwasm_std::Addr;

pub const TEST_MIX_DENOM: &str = "unym";

pub fn vk_share_fixture(index: u64) -> ContractVKShare {
    ContractVKShare {
        share: format!("share{}", index),
        announce_address: format!("localhost:{}", index),
        node_index: index,
        owner: Addr::unchecked(format!("owner{}", index)),
        verified: index % 2 == 0,
    }
}
