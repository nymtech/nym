// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::queries::{
    query_current_dealers_paged, query_dealer_details, query_past_dealers_paged,
};
use crate::dealers::transactions::try_add_dealer;
use crate::dealings::queries::query_dealings_paged;
use crate::dealings::transactions::try_commit_dealings;
use crate::epoch_state::queries::{
    query_current_epoch, query_current_epoch_threshold, query_initial_dealers,
};
use crate::epoch_state::storage::{CURRENT_EPOCH, THRESHOLD};
use crate::epoch_state::transactions::{advance_epoch_state, try_surpassed_threshold};
use crate::error::ContractError;
use crate::state::{State, MULTISIG, STATE};
use crate::verification_key_shares::queries::query_vk_shares_paged;
use crate::verification_key_shares::storage::vk_shares;
use crate::verification_key_shares::transactions::try_commit_verification_key_share;
use crate::verification_key_shares::transactions::try_verify_verification_key_share;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
    Timestamp,
};
use cw4::Cw4Contract;
use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_coconut_dkg_common::types::{Epoch, EpochId, EpochState, TimeConfiguration};
use nym_coconut_dkg_common::verification_key::ContractVKShare;

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

    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        ContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);

    let state = State {
        group_addr,
        multisig_addr,
        mix_denom: msg.mix_denom,
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
        ExecuteMsg::CommitDealing {
            dealing_bytes,
            resharing,
        } => try_commit_dealings(deps, info, dealing_bytes, resharing),
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
            idx,
            limit,
            start_after,
        } => to_binary(&query_dealings_paged(deps, idx, start_after, limit)?)?,
        QueryMsg::GetVerificationKeys {
            epoch_id,
            limit,
            start_after,
        } => to_binary(&query_vk_shares_paged(deps, epoch_id, start_after, limit)?)?,
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    CURRENT_EPOCH.save(
        deps.storage,
        &Epoch {
            state: EpochState::InProgress,
            epoch_id: 0,
            time_configuration: TimeConfiguration {
                public_key_submission_time_secs: 999999,
                dealing_exchange_time_secs: 999999,
                verification_key_submission_time_secs: 999999,
                verification_key_validation_time_secs: 999999,
                verification_key_finalization_time_secs: 999999,
                in_progress_time_secs: 999999,
            },
            finish_timestamp: env.block.time.plus_days(10000),
        },
    )?;

    let apis = [
        "https://qa-nym-api-coconut1.qa.nymte.ch/api",
        "https://qa-nym-api-coconut2.qa.nymte.ch/api",
        "https://qa-nym-api-coconut3.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-6.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-7.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-8.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-9.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-10.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-11.qa.nymte.ch/api",
        "https://qa-nym-api-coconut-12.qa.nymte.ch/api",
    ];

    let addresses = [
        Addr::unchecked("n1e6wkf0x2l4qt45uwkft9t7fs3ka02rsmft5jjn"),
        Addr::unchecked("n190f6rwtcpfgx3y84af6eapg54suehd4nzq4sh3"),
        Addr::unchecked("n1c7nu7wncuru09eg2m8429d5ff6umytet993xmf"),
        Addr::unchecked("n144fypmxc9jrdk28qrjlptpqn7h077f30vvvdjf"),
        Addr::unchecked("n15rmhp3psrnlcgp8tsa9y528ppwt0vvwuazfa4a"),
        Addr::unchecked("n18dy35dtsg7ycyp7g9pvlj29ksmjpu9h6tznxdl"),
        Addr::unchecked("n17usrtqe6ypvn3r4v05sspu3ufs30uykc7euwne"),
        Addr::unchecked("n1efpvtu5x7x4v293pc42a2fwkf265lnh05r023z"),
        Addr::unchecked("n15k2zadxm7m3wyfyx7g2uk2tctmmp43syxuqmdf"),
        Addr::unchecked("n1k4w8pp62htn0tdmrsv3gh9kzrap4tpalw8eqd7"),
    ];

    let keys = [
        "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",
    ];

    for (i, ((address, api), key)) in addresses
        .iter()
        .zip(apis.iter())
        .zip(keys.iter())
        .enumerate()
    {
        let share = ContractVKShare {
            share: key.to_string(),
            announce_address: api.to_string(),
            node_index: i as u64,
            owner: address.clone(),
            epoch_id: 0,
            verified: true,
        };

        vk_shares().save(deps.storage, (address, 0), &share)?;
    }

    THRESHOLD.save(deps.storage, &7)?;

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
