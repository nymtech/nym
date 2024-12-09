// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    ADMIN_STORAGE_KEY, CONTRACT_STATE_KEY, VERSION_HISTORY_ID_COUNTER_KEY,
    VERSION_HISTORY_NAMESPACE,
};
use cosmwasm_std::Coin;
use cosmwasm_std::{Addr, DepsMut, Env, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractState, ContractStateParams, HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry,
    OperatingCostRange, ProfitMarginRange,
};
use std::str::FromStr;

pub(crate) const CONTRACT_STATE: Item<'_, ContractState> = Item::new(CONTRACT_STATE_KEY);
pub(crate) const ADMIN: Admin = Admin::new(ADMIN_STORAGE_KEY);

pub(crate) struct NymNodeVersionHistory<'a> {
    pub(crate) id_counter: Item<'a, u32>,
    pub(crate) version_history: Map<'a, u32, HistoricalNymNodeVersion>,
}

impl NymNodeVersionHistory<'_> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            id_counter: Item::new(VERSION_HISTORY_ID_COUNTER_KEY),
            version_history: Map::new(VERSION_HISTORY_NAMESPACE),
        }
    }

    fn next_id(&self, storage: &mut dyn Storage) -> Result<u32, MixnetContractError> {
        let next = self.id_counter.may_load(storage)?.unwrap_or_default();
        self.id_counter.save(storage, &next)?;
        Ok(next)
    }

    pub fn current_version(
        &self,
        storage: &dyn Storage,
    ) -> Result<Option<HistoricalNymNodeVersionEntry>, MixnetContractError> {
        let Some(current_id) = self.id_counter.may_load(storage)? else {
            return Ok(None);
        };
        let version_information = self.version_history.load(storage, current_id)?;
        Ok(Some(HistoricalNymNodeVersionEntry {
            id: current_id,
            version_information,
        }))
    }

    pub fn insert_new(
        &self,
        storage: &mut dyn Storage,
        entry: HistoricalNymNodeVersion,
    ) -> Result<u32, MixnetContractError> {
        let next_id = self.next_id(storage)?;
        self.version_history.save(storage, next_id, &entry)?;
        Ok(next_id)
    }

    pub fn try_insert_new(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        raw_semver: &str,
    ) -> Result<u32, MixnetContractError> {
        let Ok(new_semver) = semver::Version::from_str(raw_semver) else {
            return Err(MixnetContractError::InvalidNymNodeSemver {
                provided: raw_semver.to_string(),
            });
        };

        let Some(current) = self.current_version(storage)? else {
            // treat this as genesis
            let genesis =
                HistoricalNymNodeVersion::genesis(raw_semver.to_string(), env.block.height);
            return self.insert_new(storage, genesis);
        };

        let current_semver = current.version_information.semver_unchecked();
        if new_semver <= current_semver {
            // make sure the new semver is strictly more recent than the current head
            return Err(MixnetContractError::NonIncreasingSemver {
                provided: raw_semver.to_string(),
                current: current.version_information.semver,
            });
        }

        let diff = current
            .version_information
            .difference_against_new_current(&new_semver);
        let entry = HistoricalNymNodeVersion {
            semver: raw_semver.to_string(),
            introduced_at_height: env.block.height,
            difference_since_genesis: diff,
        };
        self.insert_new(storage, entry)
    }
}

pub fn rewarding_validator_address(storage: &dyn Storage) -> Result<Addr, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_validator_address)?)
}

pub(crate) fn minimum_node_pledge(storage: &dyn Storage) -> Result<Coin, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.operators_params.minimum_pledge)?)
}

pub(crate) fn profit_margin_range(
    storage: &dyn Storage,
) -> Result<ProfitMarginRange, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.operators_params.profit_margin)?)
}

pub(crate) fn interval_operating_cost_range(
    storage: &dyn Storage,
) -> Result<OperatingCostRange, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.operators_params.interval_operating_cost)?)
}

#[allow(unused)]
pub(crate) fn minimum_delegation_stake(
    storage: &dyn Storage,
) -> Result<Option<Coin>, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.delegations_params.minimum_delegation)?)
}

pub(crate) fn rewarding_denom(storage: &dyn Storage) -> Result<String, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_denom)?)
}

pub(crate) fn vesting_contract_address(storage: &dyn Storage) -> Result<Addr, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.vesting_contract_address)?)
}

pub(crate) fn state_params(
    storage: &dyn Storage,
) -> Result<ContractStateParams, MixnetContractError> {
    Ok(CONTRACT_STATE.load(storage).map(|state| state.params)?)
}

pub(crate) fn initialise_storage(
    deps: DepsMut<'_>,
    env: &Env,
    initial_state: ContractState,
    initial_admin: Addr,
    initial_nymnode_version: String,
) -> Result<(), MixnetContractError> {
    CONTRACT_STATE.save(deps.storage, &initial_state)?;
    NymNodeVersionHistory::new().try_insert_new(deps.storage, env, &initial_nymnode_version)?;
    ADMIN.set(deps, Some(initial_admin))?;
    Ok(())
}
