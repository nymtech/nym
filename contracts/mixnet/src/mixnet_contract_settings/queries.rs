use super::storage;
use cosmwasm_std::Deps;
use mixnet_contract::{ContractSettingsParams, MixnetContractVersion, RewardingIntervalResponse};

pub(crate) fn query_contract_settings_params(deps: Deps) -> ContractSettingsParams {
    storage::read_contract_settings_params(deps.storage)
}

pub(crate) fn query_rewarding_interval(deps: Deps) -> RewardingIntervalResponse {
    let state = storage::contract_settings_read(deps.storage)
        .load()
        .unwrap();
    RewardingIntervalResponse {
        current_rewarding_interval_starting_block: state.rewarding_interval_starting_block,
        current_rewarding_interval_nonce: state.latest_rewarding_interval_nonce,
        rewarding_in_progress: state.rewarding_in_progress,
    }
}

pub(crate) fn query_contract_version() -> MixnetContractVersion {
    // as per docs
    // env! macro will expand to the value of the named environment variable at
    // compile time, yielding an expression of type `&'static str`
    // MixnetContractVersion {
    //     build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
    //     build_version: env!("VERGEN_BUILD_SEMVER").to_string(),
    //     commit_sha: env!("VERGEN_GIT_SHA").to_string(),
    //     commit_timestamp: env!("VERGEN_GIT_COMMIT_TIMESTAMP").to_string(),
    //     commit_branch: env!("VERGEN_GIT_BRANCH").to_string(),
    //     rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
    // }

    MixnetContractVersion {
        build_timestamp: ("VERGEN_BUILD_TIMESTAMP").to_string(),
        build_version: ("VERGEN_BUILD_SEMVER").to_string(),
        commit_sha: ("VERGEN_GIT_SHA").to_string(),
        commit_timestamp: ("VERGEN_GIT_COMMIT_TIMESTAMP").to_string(),
        commit_branch: ("VERGEN_GIT_BRANCH").to_string(),
        rustc_version: ("VERGEN_RUSTC_SEMVER").to_string(),
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

        storage::contract_settings(deps.as_mut().storage)
            .save(&dummy_state)
            .unwrap();

        assert_eq!(
            dummy_state.params,
            query_contract_settings_params(deps.as_ref())
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

        println!("{:?}", version);

        assert!(false);
    }
}
