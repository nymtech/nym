// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::queries::{
    query_current_dealers_paged, query_dealer_details, query_dealers_indices_paged,
    query_epoch_dealers_addresses_paged, query_epoch_dealers_paged,
    query_registered_dealer_details,
};
use crate::dealers::transactions::{
    try_add_dealer, try_transfer_ownership, try_update_announce_address,
};
use crate::dealings::queries::{
    query_dealer_dealings_status, query_dealing_chunk, query_dealing_chunk_status,
    query_dealing_metadata, query_dealing_status,
};
use crate::dealings::transactions::{try_commit_dealings_chunk, try_submit_dealings_metadata};
use crate::epoch_state::queries::{
    query_can_advance_state, query_current_epoch, query_current_epoch_threshold,
    query_epoch_at_height, query_epoch_threshold,
};
use crate::epoch_state::storage::{load_current_epoch, save_epoch};
use crate::epoch_state::transactions::{
    try_advance_epoch_state, try_force_advance_epoch_state, try_initiate_dkg,
    try_trigger_forced_reset, try_trigger_reset, try_trigger_resharing,
};
use crate::epoch_state::utils::ensure_valid_time_configuration;
use crate::error::ContractError;
use crate::state::queries::query_state;
use crate::state::storage::{DKG_ADMIN, MULTISIG, STATE};
use crate::verification_key_shares::queries::{query_vk_share, query_vk_shares_paged};
use crate::verification_key_shares::transactions::try_commit_verification_key_share;
use crate::verification_key_shares::transactions::try_verify_verification_key_share;
use cosmwasm_std::{
    entry_point, to_json_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use cw4::Cw4Contract;
use nym_coconut_dkg_common::event_attributes::{PREVIOUS_TIME_CONFIGURATION, TIME_CONFIGURATION};
use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_coconut_dkg_common::types::{Epoch, EpochState, State};
use nym_contracts_common::set_build_information;

const CONTRACT_NAME: &str = "crate:nym-coconut-dkg";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let multisig_addr = deps.api.addr_validate(&msg.multisig_addr)?;
    MULTISIG.set(deps.branch(), Some(multisig_addr.clone()))?;

    DKG_ADMIN.set(deps.branch(), Some(info.sender))?;

    let group_addr = Cw4Contract::new(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        ContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);

    let state = State {
        group_addr,
        multisig_addr,
        mix_denom: msg.mix_denom,
        key_size: msg.key_size,
    };
    STATE.save(deps.storage, &state)?;

    let time_configuration = msg.time_configuration.unwrap_or_default();
    ensure_valid_time_configuration(&time_configuration)?;
    save_epoch(
        deps.storage,
        env.block.height,
        &Epoch::new(
            EpochState::WaitingInitialisation,
            0,
            time_configuration,
            env.block.time,
        ),
    )?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitiateDkg {} => try_initiate_dkg(deps, env, info),
        ExecuteMsg::RegisterDealer {
            bte_key_with_proof,
            identity_key,
            announce_address,
            resharing,
        } => try_add_dealer(
            deps,
            env,
            info,
            bte_key_with_proof,
            identity_key,
            announce_address,
            resharing,
        ),
        ExecuteMsg::CommitDealingsMetadata {
            dealing_index,
            chunks,
            resharing,
        } => try_submit_dealings_metadata(deps, info, dealing_index, chunks, resharing),
        ExecuteMsg::CommitDealingsChunk { chunk } => {
            try_commit_dealings_chunk(deps, env, info, chunk)
        }
        ExecuteMsg::CommitVerificationKeyShare { share, resharing } => {
            try_commit_verification_key_share(deps, env, info, share, resharing)
        }
        ExecuteMsg::VerifyVerificationKeyShare {
            owner,
            resharing,
            epoch_id,
        } => try_verify_verification_key_share(deps, env, info, owner, resharing, epoch_id),
        ExecuteMsg::AdvanceEpochState {} => try_advance_epoch_state(deps, env),
        ExecuteMsg::TriggerReset {} => try_trigger_reset(deps, env, info),
        ExecuteMsg::TriggerResharing {} => try_trigger_resharing(deps, env, info),
        ExecuteMsg::TriggerForcedReset {} => try_trigger_forced_reset(deps, env, info),
        ExecuteMsg::ForceAdvanceEpochState {} => try_force_advance_epoch_state(deps, env, info),
        ExecuteMsg::TransferOwnership { transfer_to } => {
            try_transfer_ownership(deps, env, info, transfer_to)
        }
        ExecuteMsg::UpdateAnnounceAddress { new_address } => {
            try_update_announce_address(deps, info, new_address)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetState {} => to_json_binary(&query_state(deps.storage)?)?,
        QueryMsg::GetCurrentEpochState {} => to_json_binary(&query_current_epoch(deps.storage)?)?,
        QueryMsg::GetEpochStateAtHeight { height } => {
            to_json_binary(&query_epoch_at_height(deps.storage, height)?)?
        }
        QueryMsg::CanAdvanceState {} => to_json_binary(&query_can_advance_state(deps, env)?)?,
        QueryMsg::GetCurrentEpochThreshold {} => {
            to_json_binary(&query_current_epoch_threshold(deps.storage)?)?
        }
        QueryMsg::GetEpochThreshold { epoch_id } => {
            to_json_binary(&query_epoch_threshold(deps.storage, epoch_id)?)?
        }
        QueryMsg::GetRegisteredDealer {
            dealer_address,
            epoch_id,
        } => to_json_binary(&query_registered_dealer_details(
            deps,
            dealer_address,
            epoch_id,
        )?)?,
        QueryMsg::GetDealerDetails { dealer_address } => {
            to_json_binary(&query_dealer_details(deps, dealer_address)?)?
        }
        QueryMsg::GetEpochDealersAddresses {
            epoch_id,
            limit,
            start_after,
        } => to_json_binary(&query_epoch_dealers_addresses_paged(
            deps,
            epoch_id,
            start_after,
            limit,
        )?)?,
        QueryMsg::GetEpochDealers {
            epoch_id,
            limit,
            start_after,
        } => to_json_binary(&query_epoch_dealers_paged(
            deps,
            epoch_id,
            start_after,
            limit,
        )?)?,
        QueryMsg::GetCurrentDealers { limit, start_after } => {
            to_json_binary(&query_current_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDealerIndices { limit, start_after } => {
            to_json_binary(&query_dealers_indices_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDealingsMetadata {
            epoch_id,
            dealer,
            dealing_index,
        } => to_json_binary(&query_dealing_metadata(
            deps,
            epoch_id,
            dealer,
            dealing_index,
        )?)?,
        QueryMsg::GetDealerDealingsStatus { epoch_id, dealer } => {
            to_json_binary(&query_dealer_dealings_status(deps, epoch_id, dealer)?)?
        }
        QueryMsg::GetDealingStatus {
            epoch_id,
            dealer,
            dealing_index,
        } => to_json_binary(&query_dealing_status(
            deps,
            epoch_id,
            dealer,
            dealing_index,
        )?)?,
        QueryMsg::GetDealingChunkStatus {
            epoch_id,
            dealer,
            dealing_index,
            chunk_index,
        } => to_json_binary(&query_dealing_chunk_status(
            deps,
            epoch_id,
            dealer,
            dealing_index,
            chunk_index,
        )?)?,
        QueryMsg::GetDealingChunk {
            epoch_id,
            dealer,
            dealing_index,
            chunk_index,
        } => to_json_binary(&query_dealing_chunk(
            deps,
            epoch_id,
            dealer,
            dealing_index,
            chunk_index,
        )?)?,
        QueryMsg::GetVerificationKey { owner, epoch_id } => {
            to_json_binary(&query_vk_share(deps, owner, epoch_id)?)?
        }
        QueryMsg::GetVerificationKeys {
            epoch_id,
            limit,
            start_after,
        } => to_json_binary(&query_vk_shares_paged(deps, epoch_id, start_after, limit)?)?,
        QueryMsg::GetCW2ContractVersion {} => {
            to_json_binary(&cw2::get_contract_version(deps.storage)?)?
        }
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // a migrate carrying no timings changes nothing about the epoch, so it writes nothing and
    // leaves no spurious history entry
    let Some(time_configuration) = msg.time_configuration else {
        return Ok(Response::new());
    };
    ensure_valid_time_configuration(&time_configuration)?;

    // deadlines are computed per transition (`Epoch::update`), so the new timings leave the
    // running phase's deadline alone and take effect from the next transition on - and, carried
    // by `next_ceremony`, in every ceremony after this one
    let mut epoch = load_current_epoch(deps.storage)?;
    let previous = epoch.time_configuration;
    epoch.time_configuration = time_configuration;
    save_epoch(deps.storage, env.block.height, &epoch)?;

    Ok(Response::new()
        .add_attribute(PREVIOUS_TIME_CONFIGURATION, previous.to_string())
        .add_attribute(TIME_CONFIGURATION, time_configuration.to_string()))
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use crate::constants::BLOCK_TIME_FOR_VERIFICATION_SECS;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{OwnedDeps, Storage};
    use nym_coconut_dkg_common::types::{EpochId, TimeConfiguration};

    /// Stand the contract up as it will be on chain when this build is migrated in: the previous
    /// build's migration has already recorded the epoch in service, under a version older than
    /// this one.
    fn deployed_contract(
        state: EpochState,
        epoch_id: EpochId,
    ) -> OwnedDeps<impl Storage, impl cosmwasm_std::Api, impl cosmwasm_std::Querier> {
        let mut deps = init_contract();
        let env = mock_env();

        let stored = Epoch {
            keys_in_service: state.is_final().then_some(epoch_id),
            outgoing_keys: None,
            ..Epoch::new(state, epoch_id, Default::default(), env.block.time)
        };
        save_epoch(deps.as_mut().storage, env.block.height, &stored).unwrap();
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.0.1").unwrap();

        deps
    }

    fn retimed(time_configuration: TimeConfiguration) -> MigrateMsg {
        MigrateMsg {
            time_configuration: Some(time_configuration),
        }
    }

    /// The five phase durations, in order; the deprecated field only keeps the form whole.
    fn timings(phases: [u64; 5]) -> TimeConfiguration {
        #[allow(deprecated)]
        TimeConfiguration {
            public_key_submission_time_secs: phases[0],
            dealing_exchange_time_secs: phases[1],
            verification_key_submission_time_secs: phases[2],
            verification_key_validation_time_secs: phases[3],
            verification_key_finalization_time_secs: phases[4],
            in_progress_time_secs: TimeConfiguration::default().in_progress_time_secs,
        }
    }

    /// Absent timings are the deploy that changes nothing: the epoch, its recorded keys in
    /// service included, is exactly as it was.
    #[test]
    fn migrating_without_timings_leaves_the_epoch_alone() {
        let mut deps = deployed_contract(EpochState::InProgress, 4);
        let before = load_current_epoch(&deps.storage).unwrap();

        let response = migrate(deps.as_mut(), mock_env(), MigrateMsg::default()).unwrap();

        assert_eq!(before, load_current_epoch(&deps.storage).unwrap());
        assert_eq!(Some(4), before.keys_in_service);
        assert!(response.attributes.is_empty());
    }

    /// The payload replaces the stored timings and the transaction records both, in the form
    /// the CLI's `DKG_TIME_CONFIGURATION` reads. The running phase keeps its deadline: those
    /// are computed per transition, so the new timings bite from the next one.
    #[test]
    fn migrating_with_timings_rewrites_them_and_says_so() {
        let mut deps = init_contract();
        let env = mock_env();
        let mid_registration = Epoch::new(
            EpochState::PublicKeySubmission { resharing: false },
            3,
            Default::default(),
            env.block.time,
        );
        save_epoch(deps.as_mut().storage, env.block.height, &mid_registration).unwrap();
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.0.1").unwrap();

        let new_timings = timings([7200, 3600, 600, 1800, 600]);
        let response = migrate(deps.as_mut(), env.clone(), retimed(new_timings)).unwrap();

        let epoch = load_current_epoch(&deps.storage).unwrap();
        assert_eq!(
            Epoch {
                time_configuration: new_timings,
                ..mid_registration
            },
            epoch
        );

        let attribute = |key: &str| {
            response
                .attributes
                .iter()
                .find(|attribute| attribute.key == key)
                .map(|attribute| attribute.value.clone())
        };
        assert_eq!(
            attribute(PREVIOUS_TIME_CONFIGURATION),
            Some(TimeConfiguration::default().to_string())
        );
        assert_eq!(attribute(TIME_CONFIGURATION), Some(new_timings.to_string()));
    }

    /// Retiming touches the timings and nothing else about the epoch.
    #[test]
    fn migrating_with_timings_leaves_the_keys_in_service_alone() {
        let mut deps = deployed_contract(EpochState::InProgress, 4);
        let before = load_current_epoch(&deps.storage).unwrap();
        let new_timings = timings([7200, 3600, 600, 1800, 600]);

        migrate(deps.as_mut(), mock_env(), retimed(new_timings)).unwrap();

        assert_eq!(
            Epoch {
                time_configuration: new_timings,
                ..before
            },
            load_current_epoch(&deps.storage).unwrap()
        );
    }

    /// A phase with no duration would be advanceable the moment it is entered.
    #[test]
    fn migrating_with_a_zero_length_phase_is_refused() {
        let mut deps = deployed_contract(EpochState::InProgress, 0);
        let before = load_current_epoch(&deps.storage).unwrap();

        let err = migrate(
            deps.as_mut(),
            mock_env(),
            retimed(timings([3600, 3600, 600, 0, 600])),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ContractError::ZeroPhaseDuration {
                phase: "verification key validation"
            }
        ));
        assert_eq!(before, load_current_epoch(&deps.storage).unwrap());
    }

    /// A duration past the cap would not make a long phase but overflow the deadline
    /// arithmetic at the next transition, so it is refused up front.
    #[test]
    fn migrating_with_an_absurd_phase_is_refused() {
        let mut deps = deployed_contract(EpochState::InProgress, 0);
        let before = load_current_epoch(&deps.storage).unwrap();

        let err = migrate(
            deps.as_mut(),
            mock_env(),
            retimed(timings([u64::MAX, 3600, 600, 1800, 600])),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ContractError::PhaseDurationTooLong {
                phase: "public key submission",
                ..
            }
        ));
        assert_eq!(before, load_current_epoch(&deps.storage).unwrap());
    }

    /// A share committed at the start of submission has `BLOCK_TIME_FOR_VERIFICATION_SECS` to
    /// be executed by the end of finalization; timings that outlast it would strand shares.
    #[test]
    fn migrating_with_verification_phases_outlasting_proposals_is_refused() {
        let limit = BLOCK_TIME_FOR_VERIFICATION_SECS;
        let mut deps = deployed_contract(EpochState::InProgress, 0);

        // exactly the limit is one second too many
        let err = migrate(
            deps.as_mut(),
            mock_env(),
            retimed(timings([3600, 3600, limit / 2, limit / 4, limit / 4])),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ContractError::VerificationPhasesOutlastProposals { total, .. } if total == limit
        ));

        // a second under fits
        let fitting = timings([3600, 3600, limit / 2, limit / 4, limit / 4 - 1]);
        migrate(deps.as_mut(), mock_env(), retimed(fitting)).unwrap();
        assert_eq!(
            fitting,
            load_current_epoch(&deps.storage)
                .unwrap()
                .time_configuration
        );
    }

    /// Retuning is a second migrate to the same code, not a new build: the version check only
    /// refuses going backwards.
    #[test]
    fn migrating_again_at_the_same_version_retimes() {
        let mut deps = deployed_contract(EpochState::InProgress, 0);
        migrate(
            deps.as_mut(),
            mock_env(),
            retimed(timings([7200, 3600, 600, 1800, 600])),
        )
        .unwrap();

        let again = timings([1800, 1800, 600, 900, 600]);
        migrate(deps.as_mut(), mock_env(), retimed(again)).unwrap();

        assert_eq!(
            again,
            load_current_epoch(&deps.storage)
                .unwrap()
                .time_configuration
        );
    }

    /// The same bounds hold at instantiation, which used to accept anything.
    #[test]
    fn instantiating_with_invalid_timings_is_refused() {
        use crate::support::tests::fixtures::TEST_MIX_DENOM;
        use crate::support::tests::helpers::{ADMIN_ADDRESS, GROUP_CONTRACT, MULTISIG_CONTRACT};
        use cosmwasm_std::testing::{message_info, mock_dependencies};
        use cosmwasm_std::Addr;

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            group_addr: String::from(GROUP_CONTRACT),
            multisig_addr: String::from(MULTISIG_CONTRACT),
            time_configuration: Some(timings([3600, 0, 600, 1800, 600])),
            mix_denom: TEST_MIX_DENOM.to_string(),
            key_size: 5,
        };
        let err = instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
            msg,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ContractError::ZeroPhaseDuration {
                phase: "dealing exchange"
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures::TEST_MIX_DENOM;
    use crate::support::tests::helpers::{ADMIN_ADDRESS, MULTISIG_CONTRACT};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{coins, Addr};
    use cw4::Member;
    use cw_multi_test::{App, AppBuilder, AppResponse, ContractWrapper, Executor};
    use nym_coconut_dkg_common::dealing::DEFAULT_DEALINGS;
    use nym_coconut_dkg_common::msg::ExecuteMsg::{InitiateDkg, RegisterDealer};
    use nym_coconut_dkg_common::types::NodeIndex;
    use nym_group_contract_common::msg::InstantiateMsg as GroupInstantiateMsg;

    fn instantiate_with_group(app: &mut App, members: &[Addr]) -> Addr {
        let group_code_id = app.store_code(Box::new(ContractWrapper::new(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
        )));
        let msg = GroupInstantiateMsg {
            admin: Some(ADMIN_ADDRESS.to_string()),
            members: members
                .iter()
                .map(|member| Member {
                    addr: member.to_string(),
                    weight: 10,
                })
                .collect(),
        };
        let group_contract_addr = app
            .instantiate_contract(
                group_code_id,
                Addr::unchecked(ADMIN_ADDRESS),
                &msg,
                &[],
                "group",
                None,
            )
            .unwrap();

        let coconut_dkg_code_id =
            app.store_code(Box::new(ContractWrapper::new(execute, instantiate, query)));
        let msg = InstantiateMsg {
            group_addr: group_contract_addr.to_string(),
            multisig_addr: MULTISIG_CONTRACT.to_string(),
            time_configuration: None,
            mix_denom: TEST_MIX_DENOM.to_string(),
            key_size: DEFAULT_DEALINGS as u32,
        };
        app.instantiate_contract(
            coconut_dkg_code_id,
            Addr::unchecked(ADMIN_ADDRESS),
            &msg,
            &[],
            "coconut dkg",
            None,
        )
        .unwrap()
    }

    fn parse_node_index(res: AppResponse) -> NodeIndex {
        res.events
            .into_iter()
            .find(|e| &e.ty == "wasm")
            .unwrap()
            .attributes
            .into_iter()
            .find(|attr| &attr.key == "node_index")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap()
    }

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            group_addr: deps.api.addr_make("group_addr").to_string(),
            multisig_addr: deps.api.addr_make("multisig_addr").to_string(),
            time_configuration: None,
            mix_denom: "nym".to_string(),
            key_size: 5,
        };
        let info = message_info(&deps.api.addr_make("creator"), &[]);

        let res = instantiate(deps.as_mut(), env, info, msg);
        assert!(res.is_ok())
    }

    #[test]
    fn execute_add_dealer() {
        let init_funds = coins(100, TEST_MIX_DENOM);

        let api = MockApi::default();
        const MEMBER_SIZE: usize = 100;
        let members: [Addr; MEMBER_SIZE] =
            std::array::from_fn(|idx| api.addr_make(&format!("member{idx}")));

        let mut app = AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN_ADDRESS), init_funds)
                .unwrap();
        });
        let coconut_dkg_contract_addr = instantiate_with_group(&mut app, &members);

        app.execute_contract(
            Addr::unchecked(ADMIN_ADDRESS),
            coconut_dkg_contract_addr.clone(),
            &InitiateDkg {},
            &[],
        )
        .unwrap();

        for (idx, member) in members.iter().enumerate() {
            let res = app
                .execute_contract(
                    member.clone(),
                    coconut_dkg_contract_addr.clone(),
                    &RegisterDealer {
                        bte_key_with_proof: "bte_key_with_proof".to_string(),
                        identity_key: "identity".to_string(),
                        announce_address: "127.0.0.1:8000".to_string(),
                        resharing: false,
                    },
                    &[],
                )
                .unwrap();
            assert_eq!(parse_node_index(res), (idx + 1) as u64);

            let err = app
                .execute_contract(
                    member.clone(),
                    coconut_dkg_contract_addr.clone(),
                    &RegisterDealer {
                        bte_key_with_proof: "bte_key_with_proof".to_string(),
                        identity_key: "identity".to_string(),
                        announce_address: "127.0.0.1:8000".to_string(),
                        resharing: false,
                    },
                    &[],
                )
                .unwrap_err();
            assert_eq!(ContractError::AlreadyADealer, err.downcast().unwrap());
        }

        let unauthorized_member = MockApi::default().addr_make("not_a_member");
        let err = app
            .execute_contract(
                unauthorized_member,
                coconut_dkg_contract_addr,
                &RegisterDealer {
                    bte_key_with_proof: "bte_key_with_proof".to_string(),
                    identity_key: "identity".to_string(),
                    announce_address: "127.0.0.1:8000".to_string(),
                    resharing: false,
                },
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized, err.downcast().unwrap());
    }
}
