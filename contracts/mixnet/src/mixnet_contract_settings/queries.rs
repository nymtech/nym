// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract::{ContractSettingsParams, MixnetContractVersion, RewardingIntervalResponse};

pub(crate) fn query_contract_settings_params(deps: Deps) -> StdResult<ContractSettingsParams> {
    storage::CONTRACT_SETTINGS
        .load(deps.storage)
        .map(|settings| settings.params)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> StdResult<RewardingIntervalResponse> {
    let state = storage::CONTRACT_SETTINGS.load(deps.storage)?;

    Ok(RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    })
}

pub(crate) fn query_contract_version() -> MixnetContractVersion {
    // as per docs
    // env! macro will expand to the value of the named environment variable at
    // compile time, yielding an expression of type `&'static str`
    MixnetContractVersion {
        build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
        build_version: env!("VERGEN_BUILD_SEMVER").to_string(),
        commit_sha: env!("VERGEN_GIT_SHA").to_string(),
        commit_timestamp: env!("VERGEN_GIT_COMMIT_TIMESTAMP").to_string(),
        commit_branch: env!("VERGEN_GIT_BRANCH").to_string(),
        rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mixnet_contract_settings::models::ContractSettings;
    use crate::support::tests::test_helpers;

    use cosmwasm_std::Addr;

    #[test]
    fn query_for_contract_settings_works() {
        let mut deps = test_helpers::init_contract();

        let dummy_state = ContractSettings {
            owner: Addr::unchecked("someowner"),
            rewarding_validator_address: Addr::unchecked("monitor"),
            params: ContractSettingsParams {
                minimum_mixnode_bond: 123u128.into(),
                minimum_gateway_bond: 456u128.into(),
                mixnode_rewarded_set_size: 1000,
                mixnode_active_set_size: 500,
            },
            rewarding_interval_starting_block: 123,
            latest_rewarding_interval_nonce: 0,
            rewarding_in_progress: false,
        };

        storage::CONTRACT_SETTINGS
            .save(deps.as_mut().storage, &dummy_state)
            .unwrap();

        assert_eq!(
            dummy_state.params,
            query_contract_settings_params(deps.as_ref()).unwrap()
        )
    }

    #[test]
    fn query_for_contract_version_works() {
        // this basically means _something_ was grabbed from the environment at compilation time
        let version = query_contract_version();
        assert!(!version.build_timestamp.is_empty());
        assert!(!version.build_version.is_empty());
        assert!(!version.commit_sha.is_empty());
        assert!(!version.commit_timestamp.is_empty());
        assert!(!version.commit_branch.is_empty());
        assert!(!version.rustc_version.is_empty());
    }
}
