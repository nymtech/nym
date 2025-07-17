// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult};
use nym_coconut_dkg_common::dealer::PagedDealerAddressesResponse;
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::{
    msg::QueryMsg as DkgQueryMsg,
    types::{Cw4Contract, Epoch},
};
use nym_contracts_common::contract_querier::ContractQuerier;
use nym_offline_signers_common::NymOfflineSignersContractError;

pub(crate) trait DkgContractQuerier: ContractQuerier {
    fn query_dkg_cw4_contract_address(&self, dkg_contract: impl Into<String>) -> StdResult<Addr> {
        Ok(self.query_dkg_contract_state(dkg_contract)?.group_addr.0)
    }

    fn query_dkg_contract_state(
        &self,
        dkg_contract: impl Into<String>,
    ) -> StdResult<nym_coconut_dkg_common::types::State> {
        self.query_contract_storage_value(dkg_contract, b"state")?
            .ok_or(StdError::not_found(
                "unable to retrieve state information from the DKG contract storage",
            ))
    }

    fn query_dkg_epoch(&self, dkg_contract: impl Into<String>) -> StdResult<Epoch> {
        self.query_contract_storage_value(dkg_contract, b"current_epoch")?
            .ok_or(StdError::not_found(
                "unable to retrieve epoch information from the DKG contract storage",
            ))
    }

    fn query_dkg_dealers(
        &self,
        dkg_contract: impl Into<String>,
        epoch_id: EpochId,
    ) -> StdResult<Vec<Addr>> {
        let dkg_contract = dkg_contract.into();

        let mut dealers_addresses = Vec::new();
        // current max limit
        let limit = 50;
        let mut start_after = None;
        loop {
            let mut response: PagedDealerAddressesResponse = self.query_contract(
                &dkg_contract,
                &DkgQueryMsg::GetEpochDealersAddresses {
                    epoch_id,
                    limit: Some(limit),
                    start_after,
                },
            )?;

            start_after = response.start_next_after.as_ref().map(|d| d.to_string());
            if response.dealers.len() < limit as usize || response.start_next_after.is_none() {
                dealers_addresses.append(&mut response.dealers);
                // we have already exhausted the data
                break;
            } else {
                dealers_addresses.append(&mut response.dealers);
            }
        }

        Ok(dealers_addresses)
    }

    fn query_dkg_threshold(
        &self,
        dkg_contract: impl Into<String>,
        epoch_id: EpochId,
    ) -> StdResult<u64> {
        self.query_contract(dkg_contract, &DkgQueryMsg::GetEpochThreshold { epoch_id })
    }
}

impl<T> DkgContractQuerier for T where T: ContractQuerier {}

pub(crate) fn group_members(
    querier_wrapper: &QuerierWrapper,
    contract: &Cw4Contract,
) -> Result<u32, NymOfflineSignersContractError> {
    // we shouldn't ever have more group members than the default limit but IN CASE
    // something changes down the line, do go through the pagination flow
    let mut members_count = 0;

    // current max limit
    let limit = 30;
    let mut start_after = None;
    loop {
        let members = contract.list_members(querier_wrapper, start_after, Some(limit))?;
        match members.last() {
            Some(last) => {
                members_count += members.len() as u32;
                start_after = Some(last.addr.clone());

                // everything has been returned within a single query
                if members.len() < limit as usize {
                    break;
                }
            }
            None => {
                break;
            }
        }
    }

    Ok(members_count)
}
