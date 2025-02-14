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

pub(crate) const CONTRACT_STATE: Item<ContractState> = Item::new(CONTRACT_STATE_KEY);
pub(crate) const ADMIN: Admin = Admin::new(ADMIN_STORAGE_KEY);

pub(crate) struct NymNodeVersionHistory {
    pub(crate) id_counter: Item<u32>,
    pub(crate) version_history: Map<u32, HistoricalNymNodeVersion>,
}

impl NymNodeVersionHistory {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            id_counter: Item::new(VERSION_HISTORY_ID_COUNTER_KEY),
            version_history: Map::new(VERSION_HISTORY_NAMESPACE),
        }
    }

    fn next_id(&self, storage: &mut dyn Storage) -> Result<u32, MixnetContractError> {
        let id = self
            .id_counter
            .may_load(storage)?
            .map(|current| current + 1)
            .unwrap_or_default();
        self.id_counter.save(storage, &id)?;
        Ok(id)
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
        entry: &HistoricalNymNodeVersion,
    ) -> Result<u32, MixnetContractError> {
        let next_id = self.next_id(storage)?;
        self.version_history.save(storage, next_id, entry)?;
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
            return self.insert_new(storage, &genesis);
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
            .cumulative_difference_since_genesis(&new_semver);
        let entry = HistoricalNymNodeVersion {
            semver: raw_semver.to_string(),
            introduced_at_height: env.block.height,
            difference_since_genesis: diff,
        };
        self.insert_new(storage, &entry)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod nym_node_version_history {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::{mock_dependencies, mock_env};

        #[test]
        fn getting_current() -> anyhow::Result<()> {
            // empty storage
            let deps = mock_dependencies();
            let storage = NymNodeVersionHistory::new();
            assert!(storage.current_version(&deps.storage)?.is_none());

            let mut test = TestSetup::new();

            let zeroth = storage.current_version(test.storage())?.unwrap();
            let manual_zeroth = storage.version_history.load(test.storage(), 0)?;
            assert_eq!(zeroth.version_information, manual_zeroth);

            // manually update the counter to make sure data is still read correctly
            let dummy = HistoricalNymNodeVersion {
                semver: "1.2.3".to_string(),
                introduced_at_height: 1234,
                difference_since_genesis: Default::default(),
            };
            storage.id_counter.save(test.storage_mut(), &123)?;
            storage
                .version_history
                .save(test.storage_mut(), 123, &dummy)?;

            let updated = storage.current_version(test.storage())?.unwrap();
            assert_eq!(updated.version_information, dummy);

            Ok(())
        }

        #[test]
        fn inserting_new_entry() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let storage = NymNodeVersionHistory::new();

            let first = HistoricalNymNodeVersion {
                semver: "1.1.1".to_string(),
                introduced_at_height: 12,
                difference_since_genesis: Default::default(),
            };
            let second = HistoricalNymNodeVersion {
                semver: "1.1.2".to_string(),
                introduced_at_height: 123,
                difference_since_genesis: Default::default(),
            };
            let third = HistoricalNymNodeVersion {
                semver: "1.1.3".to_string(),
                introduced_at_height: 1234,
                difference_since_genesis: Default::default(),
            };

            assert_eq!(storage.id_counter.load(test.storage())?, 0);

            // id is correctly incremented for each case and no entry is overwritten
            storage.insert_new(test.storage_mut(), &first)?;
            assert_eq!(storage.id_counter.load(test.storage())?, 1);

            storage.insert_new(test.storage_mut(), &second)?;
            assert_eq!(storage.id_counter.load(test.storage())?, 2);

            storage.insert_new(test.storage_mut(), &third)?;
            assert_eq!(storage.id_counter.load(test.storage())?, 3);

            assert_eq!(storage.version_history.load(test.storage(), 1)?, first);
            assert_eq!(storage.version_history.load(test.storage(), 2)?, second);
            assert_eq!(storage.version_history.load(test.storage(), 3)?, third);

            Ok(())
        }

        #[test]
        fn inserting_initial_semver() -> anyhow::Result<()> {
            // empty storage
            let mut deps = mock_dependencies();
            let env = mock_env();
            let storage = NymNodeVersionHistory::new();

            assert!(storage
                .id_counter
                .may_load(deps.as_mut().storage)?
                .is_none());

            storage.try_insert_new(deps.as_mut().storage, &env, "1.1.1")?;
            assert_eq!(storage.id_counter.load(deps.as_mut().storage)?, 0);

            assert_eq!(
                storage
                    .version_history
                    .load(deps.as_ref().storage, 0)?
                    .semver,
                "1.1.1"
            );
            assert_eq!(
                storage
                    .current_version(deps.as_ref().storage)?
                    .unwrap()
                    .version_information
                    .semver,
                "1.1.1"
            );

            Ok(())
        }

        #[test]
        fn inserting_second_semver() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let env = test.env();
            let storage = NymNodeVersionHistory::new();

            // lower version
            assert!(storage
                .try_insert_new(test.storage_mut(), &env, "1.1.9")
                .is_err());
            assert!(storage
                .try_insert_new(test.storage_mut(), &env, "1.0.1")
                .is_err());

            // malformed
            assert!(storage
                .try_insert_new(test.storage_mut(), &env, "1.0")
                .is_err());
            assert!(storage
                .try_insert_new(test.storage_mut(), &env, "1.0bad")
                .is_err());
            assert!(storage
                .try_insert_new(test.storage_mut(), &env, "foomp")
                .is_err());

            // patch
            let mut test = TestSetup::new();
            storage.try_insert_new(test.storage_mut(), &env, "1.1.11")?;
            let current = storage
                .current_version(test.storage_mut())?
                .unwrap()
                .version_information;
            assert_eq!(current.semver, "1.1.11");
            assert_eq!(current.difference_since_genesis.major, 0);
            assert_eq!(current.difference_since_genesis.minor, 0);
            assert_eq!(current.difference_since_genesis.patch, 1);
            assert_eq!(current.difference_since_genesis.prerelease, 0);

            // minor
            let mut test = TestSetup::new();
            storage.try_insert_new(test.storage_mut(), &env, "1.2.0")?;
            let current = storage
                .current_version(test.storage_mut())?
                .unwrap()
                .version_information;
            assert_eq!(current.semver, "1.2.0");
            assert_eq!(current.difference_since_genesis.major, 0);
            assert_eq!(current.difference_since_genesis.minor, 1);
            assert_eq!(current.difference_since_genesis.patch, 0);
            assert_eq!(current.difference_since_genesis.prerelease, 0);

            // minor alt.
            let mut test = TestSetup::new();
            storage.try_insert_new(test.storage_mut(), &env, "1.2.3")?;
            let current = storage
                .current_version(test.storage_mut())?
                .unwrap()
                .version_information;
            assert_eq!(current.semver, "1.2.3");
            assert_eq!(current.difference_since_genesis.major, 0);
            assert_eq!(current.difference_since_genesis.minor, 1);
            assert_eq!(current.difference_since_genesis.patch, 0);
            assert_eq!(current.difference_since_genesis.prerelease, 0);

            Ok(())
        }

        #[test]
        fn inserting_subsequent_semver() -> anyhow::Result<()> {
            let mut test = TestSetup::new();
            let env = test.env();
            let storage = NymNodeVersionHistory::new();

            storage.try_insert_new(test.storage_mut(), &env, "1.2.0")?;
            storage.try_insert_new(test.storage_mut(), &env, "1.2.1")?;
            storage.try_insert_new(test.storage_mut(), &env, "1.2.3")?;
            let current = storage
                .current_version(test.storage_mut())?
                .unwrap()
                .version_information;
            assert_eq!(current.semver, "1.2.3");
            assert_eq!(current.difference_since_genesis.major, 0);
            assert_eq!(current.difference_since_genesis.minor, 1);
            assert_eq!(current.difference_since_genesis.patch, 3);
            assert_eq!(current.difference_since_genesis.prerelease, 0);

            storage.try_insert_new(test.storage_mut(), &env, "1.3.0")?;
            let current = storage
                .current_version(test.storage_mut())?
                .unwrap()
                .version_information;
            assert_eq!(current.semver, "1.3.0");
            assert_eq!(current.difference_since_genesis.major, 0);
            assert_eq!(current.difference_since_genesis.minor, 2);
            assert_eq!(current.difference_since_genesis.patch, 3);
            assert_eq!(current.difference_since_genesis.prerelease, 0);
            Ok(())
        }
    }
}
