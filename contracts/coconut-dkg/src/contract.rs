// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::queries::{
    query_current_dealers_paged, query_dealer_details, query_dealers_indices_paged,
    query_registered_dealer_details,
};
use crate::dealers::transactions::try_add_dealer;
use crate::dealings::queries::{
    query_dealer_dealings_status, query_dealing_chunk, query_dealing_chunk_status,
    query_dealing_metadata, query_dealing_status,
};
use crate::dealings::transactions::{try_commit_dealings_chunk, try_submit_dealings_metadata};
use crate::epoch_state::queries::{
    query_can_advance_state, query_current_epoch, query_current_epoch_threshold,
    query_epoch_threshold,
};
use crate::epoch_state::storage::{CURRENT_EPOCH, EPOCH_THRESHOLDS, THRESHOLD};
use crate::epoch_state::transactions::{
    try_advance_epoch_state, try_initiate_dkg, try_trigger_reset, try_trigger_resharing,
};
use crate::error::ContractError;
use crate::state::queries::query_state;
use crate::state::storage::{DKG_ADMIN, MULTISIG, STATE};
use crate::verification_key_shares::queries::{query_vk_share, query_vk_shares_paged};
use crate::verification_key_shares::transactions::try_commit_verification_key_share;
use crate::verification_key_shares::transactions::try_verify_verification_key_share;
use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
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

    CURRENT_EPOCH.save(
        deps.storage,
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
        ExecuteMsg::VerifyVerificationKeyShare { owner, resharing } => {
            try_verify_verification_key_share(deps, info, owner, resharing)
        }
        ExecuteMsg::AdvanceEpochState {} => try_advance_epoch_state(deps, env),
        ExecuteMsg::TriggerReset {} => try_trigger_reset(deps, env, info),
        ExecuteMsg::TriggerResharing {} => try_trigger_resharing(deps, env, info),
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps.storage)?)?,
        QueryMsg::GetCurrentEpochState {} => to_binary(&query_current_epoch(deps.storage)?)?,
        QueryMsg::CanAdvanceState {} => to_binary(&query_can_advance_state(deps.storage, env)?)?,
        QueryMsg::GetCurrentEpochThreshold {} => {
            to_binary(&query_current_epoch_threshold(deps.storage)?)?
        }
        QueryMsg::GetEpochThreshold { epoch_id } => {
            to_binary(&query_epoch_threshold(deps.storage, epoch_id)?)?
        }
        QueryMsg::GetRegisteredDealer {
            dealer_address,
            epoch_id,
        } => to_binary(&query_registered_dealer_details(
            deps,
            dealer_address,
            epoch_id,
        )?)?,
        QueryMsg::GetDealerDetails { dealer_address } => {
            to_binary(&query_dealer_details(deps, dealer_address)?)?
        }
        QueryMsg::GetCurrentDealers { limit, start_after } => {
            to_binary(&query_current_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDealerIndices { limit, start_after } => {
            to_binary(&query_dealers_indices_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDealingsMetadata {
            epoch_id,
            dealer,
            dealing_index,
        } => to_binary(&query_dealing_metadata(
            deps,
            epoch_id,
            dealer,
            dealing_index,
        )?)?,
        QueryMsg::GetDealerDealingsStatus { epoch_id, dealer } => {
            to_binary(&query_dealer_dealings_status(deps, epoch_id, dealer)?)?
        }
        QueryMsg::GetDealingStatus {
            epoch_id,
            dealer,
            dealing_index,
        } => to_binary(&query_dealing_status(
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
        } => to_binary(&query_dealing_chunk_status(
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
        } => to_binary(&query_dealing_chunk(
            deps,
            epoch_id,
            dealer,
            dealing_index,
            chunk_index,
        )?)?,
        QueryMsg::GetVerificationKey { owner, epoch_id } => {
            to_binary(&query_vk_share(deps, owner, epoch_id)?)?
        }
        QueryMsg::GetVerificationKeys {
            epoch_id,
            limit,
            start_after,
        } => to_binary(&query_vk_shares_paged(deps, epoch_id, start_after, limit)?)?,
        QueryMsg::GetCW2ContractVersion {} => to_binary(&cw2::get_contract_version(deps.storage)?)?,
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // MAINNET MIGRATION ASSERTION
    let epoch = CURRENT_EPOCH.load(deps.storage)?;
    assert_eq!(0, epoch.epoch_id);

    let threshold = THRESHOLD.load(deps.storage)?;
    EPOCH_THRESHOLDS.save(deps.storage, 0, &threshold)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures::TEST_MIX_DENOM;
    use crate::support::tests::helpers::{ADMIN_ADDRESS, MULTISIG_CONTRACT};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
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
            group_addr: "group_addr".to_string(),
            multisig_addr: "multisig_addr".to_string(),
            time_configuration: None,
            mix_denom: "nym".to_string(),
            key_size: 5,
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env, info, msg);
        assert!(res.is_ok())
    }

    #[test]
    fn execute_add_dealer() {
        let init_funds = coins(100, TEST_MIX_DENOM);
        const MEMBER_SIZE: usize = 100;
        let members: [Addr; MEMBER_SIZE] =
            std::array::from_fn(|idx| Addr::unchecked(format!("member{}", idx)));

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

        let unauthorized_member = Addr::unchecked("not_a_member");
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
