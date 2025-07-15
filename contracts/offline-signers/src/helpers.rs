// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult};
use nym_coconut_dkg_common::types::Cw4Contract;
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

    // queries CURRENT threshold value; if DKG is in progress, that's either value of the previous epoch
    // or the upcoming one if dealings had already been exchanged
    fn query_dkg_threshold(&self, dkg_contract: impl Into<String>) -> StdResult<u64> {
        self.query_contract_storage_value(dkg_contract, b"threshold")?
            .ok_or(StdError::not_found(
                "unable to retrieve threshold information from the DKG contract storage",
            ))
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
                members_count = members.len() as u32;
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
