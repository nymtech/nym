// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::queries::{
    query_current_dealers_paged, query_dealer_details, query_past_dealers_paged,
};
use crate::dealers::transactions::try_add_dealer;
use crate::dealings::queries::{query_dealing, query_dealings_paged};
use crate::dealings::transactions::try_commit_dealings;
use crate::epoch_state::queries::{
    query_current_epoch, query_current_epoch_threshold, query_initial_dealers,
};
use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::epoch_state::transactions::{advance_epoch_state, try_surpassed_threshold};
use crate::error::ContractError;
use crate::state::queries::query_state;
use crate::state::storage::{MULTISIG, STATE};
use crate::verification_key_shares::queries::query_vk_shares_paged;
use crate::verification_key_shares::transactions::try_commit_verification_key_share;
use crate::verification_key_shares::transactions::try_verify_verification_key_share;
use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use cw4::Cw4Contract;
use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_coconut_dkg_common::types::{Epoch, EpochState, State};

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let multisig_addr = deps.api.addr_validate(&msg.multisig_addr)?;
    MULTISIG.set(deps.branch(), Some(multisig_addr.clone()))?;

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
            EpochState::default(),
            0,
            msg.time_configuration.unwrap_or_default(),
            env.block.time,
        ),
    )?;

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
        ExecuteMsg::RegisterDealer {
            bte_key_with_proof,
            announce_address,
            resharing,
        } => try_add_dealer(deps, info, bte_key_with_proof, announce_address, resharing),
        ExecuteMsg::CommitDealing { dealing, resharing } => {
            try_commit_dealings(deps, info, dealing, resharing)
        }
        ExecuteMsg::CommitVerificationKeyShare { share, resharing } => {
            try_commit_verification_key_share(deps, env, info, share, resharing)
        }
        ExecuteMsg::VerifyVerificationKeyShare { owner, resharing } => {
            try_verify_verification_key_share(deps, info, owner, resharing)
        }
        ExecuteMsg::SurpassedThreshold {} => try_surpassed_threshold(deps, env),
        ExecuteMsg::AdvanceEpochState {} => advance_epoch_state(deps, env),
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps.storage)?)?,
        QueryMsg::GetCurrentEpochState {} => to_binary(&query_current_epoch(deps.storage)?)?,
        QueryMsg::GetCurrentEpochThreshold {} => {
            to_binary(&query_current_epoch_threshold(deps.storage)?)?
        }
        QueryMsg::GetInitialDealers {} => to_binary(&query_initial_dealers(deps.storage)?)?,
        QueryMsg::GetDealerDetails { dealer_address } => {
            to_binary(&query_dealer_details(deps, dealer_address)?)?
        }
        QueryMsg::GetCurrentDealers { limit, start_after } => {
            to_binary(&query_current_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetPastDealers { limit, start_after } => {
            to_binary(&query_past_dealers_paged(deps, start_after, limit)?)?
        }
        QueryMsg::GetDealing {
            epoch_id,
            dealer,
            dealing_index,
        } => to_binary(&query_dealing(deps, epoch_id, dealer, dealing_index)?)?,
        QueryMsg::GetDealings {
            epoch_id,
            dealer,
            limit,
            start_after,
        } => to_binary(&query_dealings_paged(
            deps,
            epoch_id,
            dealer,
            limit,
            start_after,
        )?)?,
        QueryMsg::GetVerificationKeys {
            epoch_id,
            limit,
            start_after,
        } => to_binary(&query_vk_shares_paged(deps, epoch_id, start_after, limit)?)?,
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
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
    use nym_coconut_dkg_common::msg::ExecuteMsg::RegisterDealer;
    use nym_coconut_dkg_common::types::{NodeIndex, TOTAL_DEALINGS};
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
            key_size: TOTAL_DEALINGS as u32,
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

        for (idx, member) in members.iter().enumerate() {
            let res = app
                .execute_contract(
                    member.clone(),
                    coconut_dkg_contract_addr.clone(),
                    &RegisterDealer {
                        bte_key_with_proof: "bte_key_with_proof".to_string(),
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
                    announce_address: "127.0.0.1:8000".to_string(),
                    resharing: false,
                },
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized, err.downcast().unwrap());
    }
}
