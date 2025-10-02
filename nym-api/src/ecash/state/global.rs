// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::helpers::{
    CachedImmutableEpochItem, IssuedCoinIndicesSignatures, IssuedExpirationDateSignatures,
};
use nym_cache::CachedImmutableItems;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::VerificationKeyAuth;
use nym_validator_client::nyxd::AccountId;
use time::Date;

pub(crate) struct GlobalEcachState {
    pub(crate) contract_address: AccountId,

    pub(crate) master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    // maybe we should use arrays here instead?
    pub(crate) coin_index_signatures: CachedImmutableEpochItem<IssuedCoinIndicesSignatures>,

    pub(crate) expiration_date_signatures:
        CachedImmutableItems<(EpochId, Date), IssuedExpirationDateSignatures>,
}

impl GlobalEcachState {
    pub(crate) fn new(contract_address: AccountId) -> Self {
        GlobalEcachState {
            contract_address,
            master_verification_key: Default::default(),
            coin_index_signatures: Default::default(),
            expiration_date_signatures: Default::default(),
        }
    }
}
