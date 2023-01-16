// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::dealer::DealerDetails;
use coconut_dkg_common::types::ContractSafeBytes;
use coconut_dkg_common::verification_key::ContractVKShare;
use cosmwasm_std::Addr;

pub const TEST_MIX_DENOM: &str = "unym";

pub fn vk_share_fixture(index: u64) -> ContractVKShare {
    ContractVKShare {
        share: format!("share{}", index),
        announce_address: format!("localhost:{}", index),
        node_index: index,
        owner: Addr::unchecked(format!("owner{}", index)),
        epoch_id: 0,
        verified: index % 2 == 0,
    }
}

pub fn dealing_bytes_fixture() -> ContractSafeBytes {
    ContractSafeBytes(vec![])
}

pub fn dealer_details_fixture(assigned_index: u64) -> DealerDetails {
    DealerDetails {
        address: Addr::unchecked(format!("owner{}", assigned_index)),
        bte_public_key_with_proof: "".to_string(),
        announce_address: "".to_string(),
        assigned_index,
    }
}
