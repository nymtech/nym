// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Uint128;

// to be determined whether those should be constants or exist as contract state
pub(crate) const MINIMUM_DEPOSIT: Uint128 = Uint128::new(1_000_000_000);

// if submitted invalid keys/signatures, get blacklisted for approximately 1 day (with ~5s per block)
pub(crate) const INVALID_ED25519_BLACKLISTING_EXPIRATION: u64 = 17280;
