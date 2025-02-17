// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ContractBuildInformation;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;

pub const CONTRACT_BUILD_INFO_STORAGE_KEY: &str = "contract_build_info";
pub const CONTRACT_BUILD_INFO: Item<ContractBuildInformation> =
    Item::new(CONTRACT_BUILD_INFO_STORAGE_KEY);

// important note. this MUST BE called inside the contract code itself and not any intermediate crate
// otherwise macro expansions will resolve to incorrect data
#[macro_export]
macro_rules! get_build_information {
    () => {
        $crate::types::ContractBuildInformation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        )
    };
}

pub fn set_state_build_information(
    store: &mut dyn Storage,
    build_info: ContractBuildInformation,
) -> StdResult<()> {
    CONTRACT_BUILD_INFO.save(store, &build_info)
}

pub fn get_contract_build_information(store: &dyn Storage) -> StdResult<ContractBuildInformation> {
    CONTRACT_BUILD_INFO.load(store)
}

#[macro_export]
macro_rules! set_build_information {
    ( $store:expr ) => {
        $crate::build_information::set_state_build_information(
            $store,
            $crate::get_build_information!(),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_build_information(&store).is_err());

        // set and get
        let contract_name = "nym-mixnet-contract";
        let contract_version = "1.2.3";
        let build_info = ContractBuildInformation::new(contract_name, contract_version);

        set_state_build_information(&mut store, build_info).unwrap();

        let loaded = get_contract_build_information(&store).unwrap();

        let expected = ContractBuildInformation {
            contract_name: contract_name.into(),
            build_version: contract_version.into(),
            build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
            commit_sha: option_env!("VERGEN_GIT_SHA")
                .unwrap_or("UNKNOWN")
                .to_string(),
            commit_timestamp: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP")
                .unwrap_or("UNKNOWN")
                .to_string(),
            commit_branch: option_env!("VERGEN_GIT_BRANCH")
                .unwrap_or("UNKNOWN")
                .to_string(),
            rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
            cargo_debug: env!("VERGEN_CARGO_DEBUG").to_string(),
            cargo_opt_level: env!("VERGEN_CARGO_OPT_LEVEL").to_string(),
        };
        assert_eq!(expected, loaded);
    }
}
