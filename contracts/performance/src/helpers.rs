// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{from_json, Binary, CustomQuery, QuerierWrapper, StdError, StdResult};
use nym_mixnet_contract_common::Interval;
use nym_performance_contract_common::EpochId;
use serde::de::DeserializeOwned;

pub(crate) trait MixnetContractQuerier {
    fn query_mixnet_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &nym_mixnet_contract_common::QueryMsg,
    ) -> StdResult<T>;

    fn query_mixnet_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>>;

    fn query_mixnet_contract_storage_value<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<T>> {
        match self.query_mixnet_contract_storage(address, key)? {
            None => Ok(None),
            Some(value) => Ok(Some(from_json(&value)?)),
        }
    }

    fn query_current_mixnet_interval(&self, address: impl Into<String>) -> StdResult<Interval> {
        self.query_mixnet_contract_storage_value(address, b"ci")?
            .ok_or(StdError::not_found(
                "unable to retrieve interval information from the mixnet contract storage",
            ))
    }

    fn query_current_absolute_mixnet_epoch_id(
        &self,
        address: impl Into<String>,
    ) -> StdResult<EpochId> {
        self.query_current_mixnet_interval(address)
            .map(|interval| interval.current_epoch_absolute_id())
    }
}

impl<C> MixnetContractQuerier for QuerierWrapper<'_, C>
where
    C: CustomQuery,
{
    fn query_mixnet_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &nym_mixnet_contract_common::QueryMsg,
    ) -> StdResult<T> {
        self.query_wasm_smart(address, msg)
    }

    fn query_mixnet_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>> {
        self.query_wasm_raw(address, key)
    }
}
