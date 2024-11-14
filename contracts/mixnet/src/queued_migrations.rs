// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod config_score_params {
    use crate::constants::CONTRACT_STATE_KEY;
    use crate::mixnet_contract_settings::storage as mixnet_params_storage;
    use cosmwasm_std::{Addr, Coin, DepsMut};
    use cw_storage_plus::Item;
    use mixnet_contract_common::error::MixnetContractError;
    use mixnet_contract_common::{
        ConfigScoreParams, ContractState, ContractStateParams, DelegationsParams, MigrateMsg,
        OperatingCostRange, OperatorsParams, ProfitMarginRange,
    };
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    pub(crate) fn add_config_score_params(
        deps: DepsMut<'_>,
        msg: &MigrateMsg,
    ) -> Result<(), MixnetContractError> {
        if semver::Version::from_str(&msg.current_nym_node_semver).is_err() {
            return Err(MixnetContractError::InvalidNymNodeSemver {
                provided: msg.current_nym_node_semver.to_string(),
            });
        }

        #[derive(Serialize, Deserialize)]
        pub struct OldContractState {
            pub owner: Option<Addr>,
            pub rewarding_validator_address: Addr,
            pub vesting_contract_address: Addr,
            pub rewarding_denom: String,
            pub params: OldContractStateParams,
        }

        #[derive(Serialize, Deserialize)]
        pub struct OldContractStateParams {
            pub minimum_delegation: Option<Coin>,
            pub minimum_pledge: Coin,
            #[serde(default)]
            pub profit_margin: ProfitMarginRange,
            #[serde(default)]
            pub interval_operating_cost: OperatingCostRange,
        }

        const OLD_CONTRACT_STATE: Item<'_, OldContractState> = Item::new(CONTRACT_STATE_KEY);
        let old_state = OLD_CONTRACT_STATE.load(deps.storage)?;

        #[allow(deprecated)]
        let new_state = ContractState {
            owner: old_state.owner,
            rewarding_validator_address: old_state.rewarding_validator_address,
            vesting_contract_address: old_state.vesting_contract_address,
            rewarding_denom: old_state.rewarding_denom,
            params: ContractStateParams {
                delegations_params: DelegationsParams {
                    minimum_delegation: old_state.params.minimum_delegation,
                },
                operators_params: OperatorsParams {
                    minimum_pledge: old_state.params.minimum_pledge,
                    profit_margin: old_state.params.profit_margin,
                    interval_operating_cost: old_state.params.interval_operating_cost,
                },
                config_score_params: ConfigScoreParams {
                    current_nym_node_semver: msg.current_nym_node_semver.to_string(),
                    version_weights: msg.version_score_weights,
                    version_score_formula_params: msg.version_score_params,
                },
            },
        };

        mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &new_state)?;
        Ok(())
    }
}

pub(crate) use config_score_params::add_config_score_params;
