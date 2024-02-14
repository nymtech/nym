// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::dealing::{DealingChunkInfo, PartialContractDealing};
use nym_coconut_dkg_common::types::ContractSafeBytes;
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

#[allow(unused)]
pub fn dealing_bytes_fixture() -> ContractSafeBytes {
    ContractSafeBytes(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
}

pub fn partial_dealing_fixture() -> PartialContractDealing {
    PartialContractDealing {
        chunk_index: 0,
        dealing_index: 0,
        data: ContractSafeBytes(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
    }
}

pub fn dealing_metadata_fixture() -> Vec<DealingChunkInfo> {
    let chunk_fixture = partial_dealing_fixture();
    vec![DealingChunkInfo {
        size: chunk_fixture.data.len() as u64,
    }]
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
