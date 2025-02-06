// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_POOL_STORAGE;
use cosmwasm_std::{Coin, Storage};
use nym_pool_contract_common::NymPoolContractError;

pub fn validate_usage_coin(storage: &dyn Storage, coin: &Coin) -> Result<(), NymPoolContractError> {
    let denom = NYM_POOL_STORAGE.pool_denomination.load(storage)?;

    if coin.amount.is_zero() {
        return Err(NymPoolContractError::EmptyUsageRequest);
    }

    if coin.denom != denom {
        return Err(NymPoolContractError::InvalidDenom {
            expected: denom,
            got: coin.denom.to_string(),
        });
    }

    Ok(())
}
