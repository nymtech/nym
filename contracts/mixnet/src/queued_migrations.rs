// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod node_version_history {
    use crate::mixnet_contract_settings::storage::NymNodeVersionHistory;
    use cosmwasm_std::DepsMut;
    use mixnet_contract_common::error::MixnetContractError;
    use mixnet_contract_common::{HistoricalNymNodeVersion, TotalVersionDifference};

    pub(crate) fn restore_node_version_history(
        deps: DepsMut<'_>,
    ) -> Result<(), MixnetContractError> {
        // sanity check:
        let storage = NymNodeVersionHistory::new();
        let Some(current) = storage.current_version(deps.storage)? else {
            return Err(MixnetContractError::FailedMigration {
                comment: "no node version history set".to_string(),
            });
        };
        if current.version_information.semver != "1.2.1"
            || current.version_information.introduced_at_height != 15902170
        {
            return Err(MixnetContractError::FailedMigration {
                comment: format!("unexpected current node version history. got: {current:?}"),
            });
        }
        let lost = HistoricalNymNodeVersion {
            semver: "1.1.12".to_string(),
            introduced_at_height: 15779133,
            difference_since_genesis: TotalVersionDifference::default(),
        };

        #[allow(clippy::unwrap_used)]
        // SAFETY: this information was already stored in the contract, so it must be a valid semver
        let difference_since_genesis = lost.cumulative_difference_since_genesis(
            &current.version_information.semver.parse().unwrap(),
        );
        let updated_current = HistoricalNymNodeVersion {
            semver: current.version_information.semver,
            introduced_at_height: current.version_information.introduced_at_height,
            difference_since_genesis,
        };

        // restore overwritten entry for 1.1.12
        storage.version_history.save(deps.storage, 0, &lost)?;

        // re-insert 1.2.1 as the current
        storage
            .version_history
            .save(deps.storage, 1, &updated_current)?;
        storage.id_counter.save(deps.storage, &1)?;

        Ok(())
    }
}

pub(crate) use node_version_history::restore_node_version_history;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet_contract_settings::queries::query_nym_node_version_history_paged;
    use crate::mixnet_contract_settings::storage::NymNodeVersionHistory;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use mixnet_contract_common::{
        HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry, TotalVersionDifference,
    };

    #[test]
    fn fixing_history_storage() -> anyhow::Result<()> {
        // current state on mainnet:
        let mut deps = mock_dependencies();
        let storage = NymNodeVersionHistory::new();

        storage.id_counter.save(deps.as_mut().storage, &0)?;
        storage.version_history.save(
            deps.as_mut().storage,
            0,
            &HistoricalNymNodeVersion {
                semver: "1.2.1".to_string(),
                introduced_at_height: 15902170,
                difference_since_genesis: Default::default(),
            },
        )?;

        // run migration
        restore_node_version_history(deps.as_mut())?;

        let current = storage.current_version(deps.as_ref().storage)?.unwrap();
        assert_eq!(current.version_information.semver, "1.2.1");
        assert_eq!(current.version_information.introduced_at_height, 15902170);
        assert_eq!(
            current.version_information.difference_since_genesis,
            TotalVersionDifference {
                major: 0,
                minor: 1,
                patch: 0,
                prerelease: 0,
            }
        );

        let history = query_nym_node_version_history_paged(deps.as_ref(), None, None)?.history;
        assert_eq!(history.len(), 2);
        assert_eq!(
            history,
            vec![
                HistoricalNymNodeVersionEntry {
                    id: 0,
                    version_information: HistoricalNymNodeVersion {
                        semver: "1.1.12".to_string(),
                        introduced_at_height: 15779133,
                        difference_since_genesis: Default::default(),
                    },
                },
                HistoricalNymNodeVersionEntry {
                    id: 1,
                    version_information: HistoricalNymNodeVersion {
                        semver: "1.2.1".to_string(),
                        introduced_at_height: 15902170,
                        difference_since_genesis: TotalVersionDifference {
                            major: 0,
                            minor: 1,
                            patch: 0,
                            prerelease: 0,
                        },
                    },
                }
            ]
        );

        let counter = storage.id_counter.load(deps.as_ref().storage)?;
        assert_eq!(counter, 1);

        // make sure adding another version doesn't mess anything up
        storage.try_insert_new(deps.as_mut().storage, &mock_env(), "1.3.0")?;

        let current = storage.current_version(deps.as_ref().storage)?.unwrap();
        assert_eq!(current.version_information.semver, "1.3.0");
        assert_eq!(
            current.version_information.difference_since_genesis,
            TotalVersionDifference {
                major: 0,
                minor: 2,
                patch: 0,
                prerelease: 0,
            }
        );
        let counter = storage.id_counter.load(deps.as_ref().storage)?;
        assert_eq!(counter, 2);

        Ok(())
    }
}
