// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::types::{ContractSafeBytes, PartialContractDealing};
use nym_coconut_dkg_common::verification_key::ContractVKShare;

pub const TEST_MIX_DENOM: &str = "unym";

pub fn vk_share_fixture(owner: &str, index: u64) -> ContractVKShare {
    ContractVKShare {
        share: format!("share{}", index),
        announce_address: format!("localhost:{}", index),
        node_index: index,
        owner: Addr::unchecked(owner),
        epoch_id: index,
        verified: index % 2 == 0,
    }
}

pub fn dealing_bytes_fixture() -> ContractSafeBytes {
    ContractSafeBytes(vec![1, 2, 3])
}

pub fn partial_dealing_fixture() -> PartialContractDealing {
    PartialContractDealing {
        index: 0,
        data: ContractSafeBytes(vec![1, 2, 3]),
    }
}

pub fn dealer_details_fixture(assigned_index: u64) -> DealerDetails {
    DealerDetails {
        address: Addr::unchecked(format!("owner{}", assigned_index)),
        bte_public_key_with_proof: "".to_string(),
        ed25519_identity: "".to_string(),
        announce_address: "".to_string(),
        assigned_index,
    }
}
