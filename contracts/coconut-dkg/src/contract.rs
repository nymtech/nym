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
    try_advance_epoch_state, try_initiate_dkg, try_trigger_forced_reset, try_trigger_reset,
    try_trigger_resharing,
};
use crate::error::ContractError;
use crate::state::queries::query_state;
use crate::state::storage::{DKG_ADMIN, MULTISIG, STATE};
use crate::verification_key_shares::queries::{query_vk_share, query_vk_shares_paged};
use crate::verification_key_shares::transactions::try_commit_verification_key_share;
use crate::verification_key_shares::transactions::try_verify_verification_key_share;
use cosmwasm_std::{
    entry_point, to_json_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Storage,
};
use cw4::Cw4Contract;
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

    save_epoch(
        deps.storage,
        env.block.height,
        &Epoch::new(
            EpochState::WaitingInitialisation,
            0,
            msg.time_configuration.unwrap_or_default(),
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
        QueryMsg::CanAdvanceState {} => {
            to_json_binary(&query_can_advance_state(deps.storage, env)?)?
        }
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
pub fn migrate(deps: DepsMut<'_>, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    seed_keys_in_service(deps.storage, env.block.height)?;

    Ok(Response::new())
}

/// Record the epoch already in service on a contract stored before [`Epoch::keys_in_service`]
/// existed.
///
/// Only an epoch that is in service is seeded, because that is the only case the chain still
/// knows the answer to: such an epoch concluded its own ceremony, so it is its own answer. Any
/// other state is left unknown rather than derived from the epoch id, which is the guess this
/// field exists to remove. Unknown refuses issuance until the next conclusion writes the truth,
/// which is the safe direction and self-correcting.
///
/// Skipped once anything is recorded: re-deriving it later would overwrite a true value, and
/// after a failed ceremony would overwrite it with precisely the wrong one.
fn seed_keys_in_service(storage: &mut dyn Storage, height: u64) -> Result<(), ContractError> {
    let epoch = load_current_epoch(storage)?;
    if epoch.keys_in_service.is_some() || !epoch.state.is_final() {
        return Ok(());
    }

    save_epoch(
        storage,
        height,
        &Epoch {
            keys_in_service: Some(epoch.epoch_id),
            // whatever this epoch superseded is unrecorded and unrecoverable, and no window can
            // be open on it: the conclusion that would have opened one predates this field
            outgoing_keys: None,
            ..epoch
        },
    )?;

    Ok(())
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{OwnedDeps, Storage};
    use nym_coconut_dkg_common::types::EpochId;

    /// Stand the contract up as it is on chain today: an epoch stored before `keys_in_service`
    /// existed, so the field reads as unknown, under a version older than this build.
    fn contract_before_the_field_existed(
        state: EpochState,
        epoch_id: EpochId,
    ) -> OwnedDeps<impl Storage, impl cosmwasm_std::Api, impl cosmwasm_std::Querier> {
        let mut deps = init_contract();
        let env = mock_env();

        let stored = Epoch {
            keys_in_service: None,
            outgoing_keys: None,
            ..Epoch::new(state, epoch_id, Default::default(), env.block.time)
        };
        save_epoch(deps.as_mut().storage, env.block.height, &stored).unwrap();
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.0.1").unwrap();

        deps
    }

    /// An epoch in service concluded its own ceremony, so seeding it is recording a fact rather
    /// than guessing one. Without this the first ceremony after the migration would run with no
    /// recorded epoch in service, and issuance would stop for its duration.
    #[test]
    fn migrating_seeds_the_epoch_already_in_service() {
        let mut deps = contract_before_the_field_existed(EpochState::InProgress, 4);

        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();

        let epoch = load_current_epoch(deps.as_ref().storage).unwrap();
        assert_eq!(Some(4), epoch.keys_in_service);
        assert_eq!(Some(4), epoch.issuing_epoch_id());
    }

    /// Mid-ceremony there is nothing to record: which epoch is in service is exactly what was
    /// never stored, and `epoch_id - 1` is the guess this whole change exists to remove. Leaving
    /// it unknown refuses issuance until the ceremony concludes and writes the truth - the
    /// behaviour from before mid-ceremony issuance existed, and the safe direction. The
    /// deployment order (contract first, while no ceremony runs) keeps it off the real path.
    #[test]
    fn migrating_mid_ceremony_records_nothing() {
        let mut deps =
            contract_before_the_field_existed(EpochState::DealingExchange { resharing: false }, 4);

        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();

        let epoch = load_current_epoch(deps.as_ref().storage).unwrap();
        assert_eq!(None, epoch.keys_in_service);
        assert_eq!(None, epoch.issuing_epoch_id());
    }

    /// A later migration must not touch what is already recorded. Re-seeding a concluded epoch
    /// would put the same value back into `keys_in_service` while clearing `outgoing_keys` along
    /// with it, dropping the window in which a collection begun under the superseded epoch can
    /// still be completed.
    #[test]
    fn migrating_again_leaves_a_recorded_epoch_alone() {
        let mut deps = init_contract();
        let env = mock_env();

        // epoch 10's keys have just come into service, superseding epoch 7's - the ceremonies
        // in between failed - so 7 is still inside its window
        let in_grace = Epoch {
            keys_in_service: Some(10),
            outgoing_keys: Some(7),
            ..Epoch::new(
                EpochState::InProgress,
                10,
                Default::default(),
                env.block.time,
            )
        };
        save_epoch(deps.as_mut().storage, env.block.height, &in_grace).unwrap();
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.0.1").unwrap();

        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();

        let epoch = load_current_epoch(deps.as_ref().storage).unwrap();
        assert_eq!(Some(10), epoch.keys_in_service);
        assert_eq!(Some(7), epoch.outgoing_keys);
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
