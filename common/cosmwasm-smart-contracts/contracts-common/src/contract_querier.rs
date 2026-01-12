// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Binary, CustomQuery, QuerierWrapper, StdResult, from_json};
use serde::Serialize;
use serde::de::DeserializeOwned;

// re-expose methods from QuerierWrapper as traits so that we could more easily define extension traits
pub trait ContractQuerier {
    fn query_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &impl Serialize,
    ) -> StdResult<T>;

    fn query_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>>;

    fn query_contract_storage_value<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<T>> {
        match self.query_contract_storage(address, key)? {
            None => Ok(None),
            Some(value) => Ok(Some(from_json(&value)?)),
        }
    }
}

impl<C> ContractQuerier for QuerierWrapper<'_, C>
where
    C: CustomQuery,
{
    fn query_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &impl Serialize,
    ) -> StdResult<T> {
        self.query_wasm_smart(address, msg)
    }

    fn query_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>> {
        self.query_wasm_raw(address, key)
    }
}
