// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
use cosmwasm_std::{Addr, Deps, QuerierWrapper, StdError, StdResult};
use nym_coconut_dkg_common::dealer::PagedDealerAddressesResponse;
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::{
    msg::QueryMsg as DkgQueryMsg,
    types::{Cw4Contract, Epoch},
};
use nym_contracts_common::contract_querier::ContractQuerier;
use nym_offline_signers_contract_common::{NymOfflineSignersContractError, SigningStatusResponse};

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

    fn query_current_dkg_epoch(&self, dkg_contract: impl Into<String>) -> StdResult<Epoch> {
        self.query_contract_storage_value(dkg_contract, b"current_epoch")?
            .ok_or(StdError::not_found(
                "unable to retrieve epoch information from the DKG contract storage",
            ))
    }

    fn query_dkg_epoch_at_height(
        &self,
        dkg_contract: impl Into<String>,
        height: u64,
    ) -> StdResult<Epoch> {
        let res: Option<Epoch> =
            self.query_contract(dkg_contract, &DkgQueryMsg::GetEpochStateAtHeight { height })?;

        res.ok_or(StdError::not_found(format!(
            "epoch hasn't been initialised/migrated to new format at height {height} yet"
        )))
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
) -> Result<Vec<Addr>, NymOfflineSignersContractError> {
    // we shouldn't ever have more group members than the default limit but IN CASE
    // something changes down the line, do go through the pagination flow
    let mut group_members = Vec::new();

    // current max limit
    let limit = 30;
    let mut start_after = None;
    loop {
        let members = contract.list_members(querier_wrapper, start_after, Some(limit))?;
        start_after = members.last().as_ref().map(|d| d.addr.clone());
        for member in &members {
            group_members.push(Addr::unchecked(&member.addr));
        }

        if members.len() < limit as usize {
            // we have already exhausted the data
            break;
        }
    }

    Ok(group_members)
}

// TODO: change our testing frameworks to allow testing this
// (the current problem is that it relies on very particular intermediate states of the DKG contract)
pub(crate) fn basic_signing_status(
    deps: Deps,
    block_height: Option<u64>,
) -> Result<SigningStatusResponse, NymOfflineSignersContractError> {
    let dkg_contract_address = NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
        .dkg_contract
        .load(deps.storage)?;

    let dkg_epoch = match block_height {
        Some(block_height) => deps
            .querier
            .query_dkg_epoch_at_height(&dkg_contract_address, block_height)?,
        None => deps
            .querier
            .query_current_dkg_epoch(&dkg_contract_address)?,
    };

    // if DKG exchange is currently in progress, retrieve dealers and threshold from the PREVIOUS epoch
    // as that'd be the set used for issuing credentials
    let epoch_id = if dkg_epoch.state.is_final() {
        dkg_epoch.epoch_id
    } else {
        dkg_epoch.epoch_id.saturating_sub(1)
    };

    let dkg_threshold = deps
        .querier
        .query_dkg_threshold(&dkg_contract_address, epoch_id)?;

    let group_contract = Cw4Contract::new(
        deps.querier
            .query_dkg_cw4_contract_address(&dkg_contract_address)?,
    );
    let total_group_members = group_members(&deps.querier, &group_contract)?.len() as u32;

    let dkg_dealers = deps
        .querier
        .query_dkg_dealers(&dkg_contract_address, epoch_id)?;

    let offline_signers = match block_height {
        Some(block_height) => NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
            .offline_signers
            .addresses
            .may_load_at_height(deps.storage, block_height)?
            .unwrap_or_default(),
        None => NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
            .offline_signers
            .addresses
            .load(deps.storage)?,
    }
    .into_iter()
    .filter(|offline_signer| dkg_dealers.contains(offline_signer))
    .count() as u32;

    let available_signers = (dkg_dealers.len() as u32).saturating_sub(offline_signers);

    Ok(SigningStatusResponse {
        dkg_epoch_id: epoch_id,
        signing_threshold: dkg_threshold,
        total_group_members,
        current_registered_dealers: dkg_dealers.len() as u32,
        offline_signers,
        threshold_available: available_signers as u64 >= dkg_threshold,
    })
}

#[cfg(test)]
mod tests {
    use crate::helpers::group_members;
    use crate::testing::init_contract_tester_with_group_members;
    use cw4::Cw4Contract;
    use nym_coconut_dkg::testable_dkg_contract::GroupContract;
    use nym_contracts_common_testing::ContractOpts;

    #[test]
    fn getting_group_members() -> anyhow::Result<()> {
        for members in [0, 10, 100, 1000] {
            let tester = init_contract_tester_with_group_members(members);
            let group_contract =
                Cw4Contract::new(tester.unchecked_contract_address::<GroupContract>());
            let querier = tester.deps().querier;

            let addresses = group_members(&querier, &group_contract)?;
            assert_eq!(addresses.len(), members);
        }

        Ok(())
    }
}
