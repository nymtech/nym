// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::queries::query_admin;
use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
use crate::transactions::{
    try_handle_node_unbonding, try_relay_self_declarations, try_remove_entries,
    try_remove_override, try_remove_whitelisted_agent, try_set_override,
    try_set_whitelisted_agent, try_submit_measurements, try_update_config,
    try_update_contract_admin,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_geolocation_contract_common::{
    ExecuteMsg, GeolocationContractError, InstantiateMsg, MigrateMsg, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-geolocation-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, GeolocationContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;
    let config = msg.initial_contract_config();
    GEOLOCATION_CONTRACT_STORAGE.initialise(
        deps,
        info.sender,
        mixnet_contract_address,
        msg.initial_whitelist,
        config,
    )?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, GeolocationContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::SubmitMeasurements { measurements } => {
            try_submit_measurements(deps, env, info, measurements)
        }
        ExecuteMsg::RelaySelfDeclarations { declarations } => {
            try_relay_self_declarations(deps, env, info, declarations)
        }
        ExecuteMsg::SetOverride { subject, payload } => {
            try_set_override(deps, env, info, subject, payload)
        }
        ExecuteMsg::RemoveOverride { subject } => try_remove_override(deps, info, subject),
        ExecuteMsg::SetWhitelistedAgent { agent, permissions } => {
            try_set_whitelisted_agent(deps, info, agent, permissions)
        }
        ExecuteMsg::RemoveWhitelistedAgent { agent } => {
            try_remove_whitelisted_agent(deps, info, agent)
        }
        ExecuteMsg::RemoveEntries { keys } => try_remove_entries(deps, info, keys),
        ExecuteMsg::UpdateConfig {
            max_skew_secs,
            max_batch_size,
            max_payload_size,
        } => try_update_config(deps, info, max_skew_secs, max_batch_size, max_payload_size),
        ExecuteMsg::OnNymNodeUnbond { node_id } => try_handle_node_unbonding(deps, info, node_id),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, GeolocationContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::Config {} => todo!(),
        QueryMsg::Entry { subject, source } => todo!(),
        QueryMsg::SubjectEntries { subject } => todo!(),
        QueryMsg::NymNodeEntries { node_id } => todo!(),
        QueryMsg::SubjectMeasurements { subject } => todo!(),
        QueryMsg::AllRecords { start_after, limit } => todo!(),
        QueryMsg::Digest {} => todo!(),
        QueryMsg::Whitelist {} => todo!(),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, GeolocationContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiation {
        use super::*;
        use crate::storage::{assert_digest_is_refold, GEOLOCATION_CONTRACT_STORAGE};
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
        use nym_geolocation_contract_common::constants::{
            DEFAULT_MAX_BATCH_SIZE, DEFAULT_MAX_PAYLOAD_SIZE, DEFAULT_MAX_SKEW_SECS,
        };
        use nym_geolocation_contract_common::{AgentPermissions, ContractConfig, InitialAgent};
        use nym_lthash::LtHash16;

        fn init_message(api: &MockApi) -> InstantiateMsg {
            InstantiateMsg {
                mixnet_contract_address: api.addr_make("mixnet-contract").to_string(),
                initial_whitelist: vec![],
                max_skew_secs: None,
                max_batch_size: None,
                max_payload_size: None,
            }
        }

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = init_message(&deps.api);

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            GEOLOCATION_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }

        #[test]
        fn stores_the_mixnet_contract_address() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = init_message(&deps.api);

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE
                    .mixnet_contract_address
                    .load(&deps.storage)?,
                deps.api.addr_make("mixnet-contract")
            );

            Ok(())
        }

        #[test]
        fn an_unparseable_mixnet_contract_address_is_rejected() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mut init_msg = init_message(&deps.api);
            init_msg.mixnet_contract_address = "not-an-address".to_owned();

            let some_sender = deps.api.addr_make("some_sender");
            let res = instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            );

            assert!(res.is_err());
        }

        #[test]
        fn every_omitted_tunable_falls_back_to_its_default() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = init_message(&deps.api);

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE.config.load(&deps.storage)?,
                ContractConfig {
                    max_skew_secs: DEFAULT_MAX_SKEW_SECS,
                    max_batch_size: DEFAULT_MAX_BATCH_SIZE,
                    max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
                }
            );

            Ok(())
        }

        #[test]
        fn explicitly_provided_tunables_are_honoured() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mut init_msg = init_message(&deps.api);
            init_msg.max_skew_secs = Some(30);
            init_msg.max_batch_size = Some(7);
            init_msg.max_payload_size = Some(4096);

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE.config.load(&deps.storage)?,
                ContractConfig {
                    max_skew_secs: 30,
                    max_batch_size: 7,
                    max_payload_size: 4096,
                }
            );

            Ok(())
        }

        #[test]
        fn a_configuration_that_could_accept_no_write_is_rejected() {
            // neither zero is permanent, since the admin can update either, but neither
            // announces itself: the contract would instantiate, keep answering queries, and
            // reject every agent submission until somebody worked out why
            for make_inert in [
                |msg: &mut InstantiateMsg| msg.max_batch_size = Some(0),
                |msg: &mut InstantiateMsg| msg.max_payload_size = Some(0),
            ] {
                let mut deps = mock_dependencies();
                let mut init_msg = init_message(&deps.api);
                make_inert(&mut init_msg);

                let some_sender = deps.api.addr_make("some_sender");
                let res = instantiate(
                    deps.as_mut(),
                    mock_env(),
                    message_info(&some_sender, &[]),
                    init_msg,
                );

                assert!(matches!(
                    res,
                    Err(GeolocationContractError::InvalidConfig { .. })
                ));
            }

            // a zero skew is a different thing entirely and stays accepted: it admits no
            // clock drift at all, which is a strict policy rather than an inert contract
            let mut deps = mock_dependencies();
            let mut init_msg = init_message(&deps.api);
            init_msg.max_skew_secs = Some(0);

            let some_sender = deps.api.addr_make("some_sender");
            assert!(instantiate(
                deps.as_mut(),
                mock_env(),
                message_info(&some_sender, &[]),
                init_msg,
            )
            .is_ok());
        }

        #[test]
        fn the_initial_whitelist_is_stored_and_committed_to_the_digest() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let mut init_msg = init_message(&deps.api);
            init_msg.initial_whitelist = vec![
                InitialAgent {
                    agent: deps.api.addr_make("agent1").to_string(),
                    permissions: AgentPermissions {
                        can_measure: true,
                        can_relay_self_declared: false,
                    },
                },
                InitialAgent {
                    agent: deps.api.addr_make("agent2").to_string(),
                    permissions: AgentPermissions {
                        can_measure: false,
                        can_relay_self_declared: true,
                    },
                },
            ];
            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            let whitelisted = GEOLOCATION_CONTRACT_STORAGE.all_whitelisted_agents(&deps.storage)?;
            assert_eq!(whitelisted.len(), 2);
            assert_eq!(
                GEOLOCATION_CONTRACT_STORAGE
                    .may_load_agent_permissions(&deps.storage, &deps.api.addr_make("agent1"))?,
                Some(AgentPermissions {
                    can_measure: true,
                    can_relay_self_declared: false
                })
            );

            // the whitelist is a digest-committed entry class, not configuration: read-time
            // authorisation is only sound if a client can verify which writers were authorised
            assert_digest_is_refold(&deps.storage);
            assert_ne!(
                GEOLOCATION_CONTRACT_STORAGE.load_digest(&deps.storage)?,
                LtHash16::new(),
                "initial agents must actually move the digest"
            );

            Ok(())
        }
    }
}
