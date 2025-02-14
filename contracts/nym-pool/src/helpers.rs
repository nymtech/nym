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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::NymPoolStorage;
    use crate::testing::TestSetup;
    use cosmwasm_std::coin;

    #[test]
    fn validating_coin_usage() -> anyhow::Result<()> {
        let test = TestSetup::init();
        let storage = NymPoolStorage::new();
        let denom = storage.pool_denomination.load(test.storage())?;

        // amount has to be non-zero
        assert_eq!(
            validate_usage_coin(test.storage(), &coin(0, &denom)).unwrap_err(),
            NymPoolContractError::EmptyUsageRequest
        );

        // denom has to match the value set in the storage
        assert_eq!(
            validate_usage_coin(test.storage(), &coin(1000, "bad-denom")).unwrap_err(),
            NymPoolContractError::InvalidDenom {
                expected: denom.to_string(),
                got: "bad-denom".to_string(),
            }
        );

        assert!(validate_usage_coin(test.storage(), &coin(1000, denom)).is_ok());

        Ok(())
    }
}
