// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::verification_key::VerificationKeyShare;

#[cw_serde]
pub struct FakeDealerData {
    pub vk: VerificationKeyShare,
    pub ed25519_identity: String,
    pub announce: String,
    pub owner: Addr,
}

#[cw_serde]
pub struct MigrateMsg {
    pub dealers: Vec<FakeDealerData>,
}
